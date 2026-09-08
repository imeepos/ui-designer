import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ApiError,
  type ApiAdapter,
  type BoardBrief,
  type CallOptions,
  type Candidate,
  type CreateProjectInput,
  type DeleteTarget,
  type ExportResult,
  type GenerateOptions,
  type GenerateResult,
  type ProjectDetail,
  type ProjectSummary,
} from "@/lib/api/types";

/**
 * Tauri-backed adapter: invokes the Rust commands that wrap rudder-core::ops
 * (docs/ARCHITECTURE.md §7). DTOs carry absolute file paths; `convertFileSrc`
 * turns them into asset-protocol URLs at map time.
 *
 * Rust side errors arrive as `{code, message, hint}` envelopes whose `code`
 * already uses the frontend `ApiErrorCode` vocabulary.
 *
 * Progress: every generate/export call gets a `jobId`; the Rust side emits
 * `rudder://job` events with started/finished/failed phases while this adapter
 * ramps 0→0.9 on a timer (a real generation takes 30s-3min) and snaps to 1 on
 * `finished`. Without an `onProgress` callback no listener is set up.
 */

// ---------------------------------------------------------------------------
// Rust DTO mirrors (src-tauri/src/view.rs, serde camelCase)
// ---------------------------------------------------------------------------

export interface CanvasDto {
  w: number;
  h: number;
  preset: string;
}

export interface CandidateDto {
  id: string;
  path: string;
  createdAt: number;
  seed?: number;
}

export interface CurrentDto {
  path: string;
  candidateId: string;
}

export interface HistoryDto {
  ts: number;
  path: string;
}

export interface AnchorDto {
  candidateId: string;
  path: string;
  createdAt: number;
}

export interface PageDto {
  slug: string;
  brief: string;
  candidates: CandidateDto[];
  current: CurrentDto | null;
  history: HistoryDto[];
  updatedAt: number;
}

export interface ComponentDto {
  name: string;
  type: string;
  brief: string;
  candidates: CandidateDto[];
  current: CurrentDto | null;
  history: HistoryDto[];
  updatedAt: number;
}

export interface ProjectSummaryDto {
  id: string;
  name: string;
  size: CanvasDto;
  createdAt: number;
  hasAnchor: boolean;
  pageCount: number;
  componentCount: number;
}

export interface ProjectDetailDto {
  id: string;
  name: string;
  size: CanvasDto;
  brandBrief: string;
  styleBrief: string;
  anchor: AnchorDto | null;
  boardCandidates: CandidateDto[];
  pages: PageDto[];
  components: ComponentDto[];
  createdAt: number;
}

export interface GeneratePayloadDto {
  candidates: CandidateDto[];
  project: ProjectDetailDto;
}

export interface ExportFileDto {
  path: string;
  bytes: number;
  kind: string;
}

export interface ExportResultDto {
  outDir: string;
  files: ExportFileDto[];
}

export interface BoardBriefDto {
  brandKeywords: string;
  colorDirection: string;
  fontMood: string;
  radiusDensity: string;
  reference: string;
}

/** Payload of the `rudder://job` progress events. */
export interface JobEventPayload {
  jobId: string;
  phase: "started" | "finished" | "failed";
  kind: string;
  target: string;
  message?: string;
}

// ---------------------------------------------------------------------------
// Progress constants (real generations take 30s-3min)
// ---------------------------------------------------------------------------

const JOB_EVENT = "rudder://job";
const RAMP_TICK_MS = 200;
const RAMP_CAP = 0.9;
const EXPECTED_GENERATION_MS = 120_000;

export type InvokeFn = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

/** API error codes the Rust bridge can send (types.ts `ApiErrorCode`). */
const KNOWN_CODES = new Set([
  "VALIDATION_ERROR",
  "ANCHOR_REQUIRED",
  "ANCHOR_LOCKED",
  "NOT_FOUND",
  "DUPLICATE_SLUG",
  "DUPLICATE_NAME",
  "CANCELLED",
  "EXPORT_FAILED",
  "API_ERROR",
  "NOT_IMPLEMENTED",
  "UNKNOWN",
]);

function toApiError(error: unknown): ApiError {
  if (error instanceof ApiError) return error;
  if (error && typeof error === "object" && "code" in error) {
    const raw = error as { code?: unknown; message?: unknown; hint?: unknown };
    const code = typeof raw.code === "string" && KNOWN_CODES.has(raw.code) ? raw.code : "UNKNOWN";
    return new ApiError(
      code as ApiError["code"],
      typeof raw.message === "string" ? raw.message : "Unknown error",
      typeof raw.hint === "string" ? raw.hint : undefined,
    );
  }
  return new ApiError(
    "UNKNOWN",
    error instanceof Error ? error.message : String(error),
  );
}

function newJobId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `job-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

function mapCandidate(toUrl: (path: string) => string, dto: CandidateDto): Candidate {
  return {
    seed: dto.seed,
    id: dto.id,
    url: toUrl(dto.path),
    createdAt: dto.createdAt,
  };
}

function mapProject(toUrl: (path: string) => string, dto: ProjectDetailDto): ProjectDetail {
  return {
    id: dto.id,
    name: dto.name,
    size: { ...dto.size } as ProjectDetail["size"],
    brandBrief: dto.brandBrief,
    styleBrief: dto.styleBrief,
    anchor: dto.anchor
      ? {
          candidateId: dto.anchor.candidateId,
          url: toUrl(dto.anchor.path),
          createdAt: dto.anchor.createdAt,
        }
      : null,
    boardCandidates: dto.boardCandidates.map((candidate) => mapCandidate(toUrl, candidate)),
    pages: dto.pages.map((page) => ({
      slug: page.slug,
      brief: page.brief,
      candidates: page.candidates.map((candidate) => mapCandidate(toUrl, candidate)),
      current: page.current
        ? { url: toUrl(page.current.path), candidateId: page.current.candidateId }
        : null,
      history: page.history.map((entry) => ({ ts: entry.ts, url: toUrl(entry.path) })),
      updatedAt: page.updatedAt,
    })),
    components: dto.components.map((component) => ({
      name: component.name,
      type: component.type as ProjectDetail["components"][number]["type"],
      brief: component.brief,
      candidates: component.candidates.map((candidate) => mapCandidate(toUrl, candidate)),
      current: component.current
        ? { url: toUrl(component.current.path), candidateId: component.current.candidateId }
        : null,
      history: component.history.map((entry) => ({ ts: entry.ts, url: toUrl(entry.path) })),
      updatedAt: component.updatedAt,
    })),
    createdAt: dto.createdAt,
  };
}

/**
 * Real desktop adapter. Enabled when a Tauri runtime is detected
 * (`createApi`); constructor injection keeps it unit-testable in vitest.
 */
export class TauriApi implements ApiAdapter {
  readonly mode = "tauri" as const;

  constructor(
    private readonly invokeFn: InvokeFn = invoke,
    private readonly toUrl: (path: string) => string = (path) => convertFileSrc(path),
  ) {}

  private async call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    try {
      return await this.invokeFn<T>(command, args);
    } catch (error) {
      throw toApiError(error);
    }
  }

  /** Reject with CANCELLED when the caller aborts (the Rust job keeps running). */
  private withAbort<T>(promise: Promise<T>, signal?: AbortSignal): Promise<T> {
    if (!signal) return promise;
    return new Promise<T>((resolve, reject) => {
      if (signal.aborted) {
        reject(new ApiError("CANCELLED", "Operation cancelled"));
        return;
      }
      const onAbort = () => reject(new ApiError("CANCELLED", "Operation cancelled"));
      signal.addEventListener("abort", onAbort, { once: true });
      promise.then(
        (value) => {
          signal.removeEventListener("abort", onAbort);
          resolve(value);
        },
        (error) => {
          signal.removeEventListener("abort", onAbort);
          reject(error);
        },
      );
    });
  }

  /**
   * Drive `onProgress` for one job: subscribe to `rudder://job`, ramp
   * 0→0.9 over ~2min, snap to 1 on `finished`. Returns a stop function.
   */
  private trackJob(jobId: string, options?: GenerateOptions): () => void {
    const onProgress = options?.onProgress;
    if (!onProgress) return () => {};

    let stopped = false;
    let startedAt = Date.now();
    let unlisten: (() => void) | null = null;
    const timer = setInterval(() => {
      if (stopped) return;
      const elapsed = Date.now() - startedAt;
      onProgress(Math.min(RAMP_CAP, (elapsed / EXPECTED_GENERATION_MS) * RAMP_CAP));
    }, RAMP_TICK_MS);
    const stop = () => {
      stopped = true;
      clearInterval(timer);
      unlisten?.();
    };

    // Phase events (best effort): `started` restarts the ramp clock,
    // `finished`/`failed` stop it. Progress itself comes from the ramp.
    // Outside a Tauri webview `listen` rejects; the ramp still runs.
    void listen<JobEventPayload>(JOB_EVENT, (event) => {
      if (stopped || event.payload.jobId !== jobId) return;
      if (event.payload.phase === "started") {
        startedAt = Date.now();
      } else if (event.payload.phase === "finished") {
        onProgress(1);
        stop();
      } else if (event.payload.phase === "failed") {
        stop();
      }
    })
      .then((dispose) => {
        if (stopped) dispose();
        else unlisten = dispose;
      })
      .catch(() => {
        // Non-Tauri runtime (tests/browser preview): ignore.
      });
    return stop;
  }

  private async runGenerate(
    command: string,
    args: Record<string, unknown>,
    options?: GenerateOptions,
  ): Promise<GenerateResult> {
    const jobId = newJobId();
    const stopJob = this.trackJob(jobId, options);
    try {
      const dto = await this.withAbort(
        this.call<GeneratePayloadDto>(command, {
          ...args,
          options: { count: options?.count, jobId },
        }),
        options?.signal,
      );
      return {
        candidates: dto.candidates.map((candidate) => mapCandidate(this.toUrl, candidate)),
        project: mapProject(this.toUrl, dto.project),
      };
    } finally {
      stopJob();
    }
  }

  async createProject(input: CreateProjectInput, _options?: CallOptions): Promise<ProjectDetail> {
    const dto = await this.call<ProjectDetailDto>("create_project", {
      input: {
        name: input.name,
        size: input.size,
        brandBrief: input.brandBrief,
        styleBrief: input.styleBrief ?? "",
      },
    });
    return mapProject(this.toUrl, dto);
  }

  async listProjects(_options?: CallOptions): Promise<ProjectSummary[]> {
    const rows = await this.call<ProjectSummaryDto[]>("list_projects");
    return rows.map((row) => ({
      id: row.id,
      name: row.name,
      size: { ...row.size } as ProjectSummary["size"],
      createdAt: row.createdAt,
      hasAnchor: row.hasAnchor,
      pageCount: row.pageCount,
      componentCount: row.componentCount,
    }));
  }

  async getProject(id: string, _options?: CallOptions): Promise<ProjectDetail> {
    const dto = await this.call<ProjectDetailDto>("get_project", { projectId: id });
    return mapProject(this.toUrl, dto);
  }

  async generateBoard(
    projectId: string,
    brief: BoardBrief,
    options?: GenerateOptions,
  ): Promise<GenerateResult> {
    const briefDto: BoardBriefDto = {
      brandKeywords: brief.brandKeywords,
      colorDirection: brief.colorDirection,
      fontMood: brief.fontMood,
      radiusDensity: brief.radiusDensity,
      reference: brief.reference,
    };
    return this.runGenerate("generate_board", { projectId, brief: briefDto }, options);
  }

  async pickAnchor(
    projectId: string,
    candidateId: string,
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    const dto = await this.call<ProjectDetailDto>("pick_anchor", { projectId, candidateId });
    return mapProject(this.toUrl, dto);
  }

  async addPage(
    projectId: string,
    input: { slug: string; brief: string },
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    const dto = await this.call<ProjectDetailDto>("add_page", { projectId, input });
    return mapProject(this.toUrl, dto);
  }

  async generatePage(
    projectId: string,
    slug: string,
    options?: GenerateOptions,
  ): Promise<GenerateResult> {
    return this.runGenerate("generate_page", { projectId, slug }, options);
  }

  async pickPage(
    projectId: string,
    slug: string,
    candidateId: string,
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    const dto = await this.call<ProjectDetailDto>("pick_page", { projectId, slug, candidateId });
    return mapProject(this.toUrl, dto);
  }

  async addComponent(
    projectId: string,
    input: { name: string; type: string; brief: string },
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    const dto = await this.call<ProjectDetailDto>("add_component", { projectId, input });
    return mapProject(this.toUrl, dto);
  }

  async generateComponent(
    projectId: string,
    name: string,
    options?: GenerateOptions,
  ): Promise<GenerateResult> {
    return this.runGenerate("generate_component", { projectId, name }, options);
  }

  async pickComponent(
    projectId: string,
    name: string,
    candidateId: string,
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    const dto = await this.call<ProjectDetailDto>("pick_component", {
      projectId,
      name,
      candidateId,
    });
    return mapProject(this.toUrl, dto);
  }

  async exportProject(
    projectId: string,
    outDir: string,
    options?: GenerateOptions,
  ): Promise<ExportResult> {
    const jobId = newJobId();
    const stopJob = this.trackJob(jobId, options);
    try {
      const dto = await this.withAbort(
        this.call<ExportResultDto>("export_project", {
          projectId,
          outDir,
          options: { jobId },
        }),
        options?.signal,
      );
      return {
        outDir: dto.outDir,
        files: dto.files.map((file) => ({
          path: file.path,
          bytes: file.bytes,
          kind: file.kind as ExportResult["files"][number]["kind"],
        })),
      };
    } finally {
      stopJob();
    }
  }

  async deleteArtifact(
    projectId: string,
    target: DeleteTarget,
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    const dto = await this.call<ProjectDetailDto>("delete_artifact", { projectId, target });
    return mapProject(this.toUrl, dto);
  }
}
