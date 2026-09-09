// Domain types + the ApiAdapter contract shared by MockApi and TauriApi.
// Errors are always {code, message, hint}; user-visible copy is resolved
// through i18n keys (`errors.<code>.*`), never from these raw strings.

export type SizePreset = "web" | "mobile" | "desktop" | "custom";

export interface CanvasSize {
  w: number;
  h: number;
  preset: SizePreset;
}

export type ComponentType =
  | "buttons"
  | "forms"
  | "cards"
  | "navigation"
  | "icons"
  | "tables"
  | "modals";

export const COMPONENT_TYPES: ComponentType[] = [
  "buttons",
  "forms",
  "cards",
  "navigation",
  "icons",
  "tables",
  "modals",
];

/** Visual variant hint for mock fixtures; real adapters return undefined. */
export interface Candidate {
  id: string;
  url: string;
  createdAt: number;
  seed?: number;
  /** CSS filter that simulates a distinct draft (mock only). */
  filter?: string;
}

export interface HistoryEntry {
  ts: number;
  url: string;
  filter?: string;
}

export interface PageItem {
  slug: string;
  brief: string;
  candidates: Candidate[];
  current: { url: string; candidateId: string; filter?: string } | null;
  history: HistoryEntry[];
  updatedAt: number;
}

export interface ComponentItem {
  name: string;
  type: ComponentType;
  brief: string;
  candidates: Candidate[];
  current: { url: string; candidateId: string; filter?: string } | null;
  history: HistoryEntry[];
  updatedAt: number;
}

export interface AnchorRef {
  candidateId: string;
  url: string;
  filter?: string;
  createdAt: number;
}

export interface ProjectSummary {
  id: string;
  name: string;
  size: CanvasSize;
  createdAt: number;
  hasAnchor: boolean;
  /** Anchor board image for the home card preview; null → compass empty state. */
  anchorUrl: string | null;
  pageCount: number;
  componentCount: number;
}

export interface ProjectDetail {
  id: string;
  name: string;
  size: CanvasSize;
  brandBrief: string;
  styleBrief: string;
  anchor: AnchorRef | null;
  boardCandidates: Candidate[];
  pages: PageItem[];
  components: ComponentItem[];
  createdAt: number;
}

export interface CreateProjectInput {
  name: string;
  size: CanvasSize;
  brandBrief: string;
  styleBrief?: string;
}

export interface BoardBrief {
  brandKeywords: string;
  colorDirection: string;
  fontMood: string;
  radiusDensity: string;
  reference: string;
}

export interface GenerateOptions {
  count?: number;
  /** Generation quality tier; default low (exploration). */
  quality?: "low" | "medium" | "high";
  signal?: AbortSignal;
  /** 0..1 progress feedback for loading UI (mock simulates it). */
  onProgress?: (progress: number) => void;
}

export interface GenerateResult {
  candidates: Candidate[];
  project: ProjectDetail;
}

export interface ExportResult {
  outDir: string;
  files: ExportedFile[];
}

export interface ExportedFile {
  path: string;
  bytes: number;
  kind: "image" | "manifest" | "prompts" | "template";
}

export type DeleteTarget =
  | { kind: "boardCandidate"; candidateId: string }
  | { kind: "page"; slug: string }
  | { kind: "pageCurrent"; slug: string }
  | { kind: "pageHistory"; slug: string; ts: number }
  | { kind: "component"; name: string }
  | { kind: "componentCurrent"; name: string }
  | { kind: "componentHistory"; name: string; ts: number };

/** Generation params as recorded per batch (project.json GenRecord). */
export interface GenParams {
  model: string;
  /** `"1536x1024"` form. */
  size: string;
  quality: string;
  n: number;
  seed?: number;
  thinking?: string;
}

/** One real generation batch — the lineage behind a produced image. */
export interface GenRecord {
  /** RFC 3339 timestamp of the batch. */
  at: string;
  /** `generations` | `edits`. */
  endpoint: string;
  prompt: string;
  params: GenParams;
  candidateIds: string[];
  /** `engine` (assembled) | `agent-file` (proxy prompt file); absent on pre-v0.2 records. */
  source?: "engine" | "agent-file";
  templateId?: string;
}

/**
 * The stage object a lineage lookup targets: the anchor (overview), a page
 * current or a component current, identified by its candidate id.
 */
export type LineageTarget =
  | { kind: "board"; candidateId: string }
  | { kind: "page"; slug: string; candidateId: string }
  | { kind: "component"; name: string; candidateId: string };

/** Unified error contract: {code, message, hint}. */
export type ApiErrorCode =
  | "VALIDATION_ERROR"
  | "ANCHOR_REQUIRED"
  | "ANCHOR_LOCKED"
  | "NOT_FOUND"
  | "DUPLICATE_SLUG"
  | "DUPLICATE_NAME"
  | "NO_CREDENTIALS"
  | "KEYCHAIN_ACCESS"
  | "CANCELLED"
  | "EXPORT_FAILED"
  | "API_ERROR"
  | "NOT_IMPLEMENTED"
  | "UNKNOWN";

export class ApiError extends Error {
  readonly code: ApiErrorCode;
  readonly hint?: string;
  /** i18n interpolation params, e.g. {min:"1024"} for size errors. */
  readonly params?: Record<string, string | number>;

  constructor(
    code: ApiErrorCode,
    message: string,
    hint?: string,
    params?: Record<string, string | number>,
  ) {
    super(message);
    this.name = "ApiError";
    this.code = code;
    this.hint = hint;
    this.params = params;
  }
}

export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError;
}

/** Common call options for every adapter method. */
export interface CallOptions {
  signal?: AbortSignal;
}

/**
 * Adapter contract for the whole four-step flow. MockApi implements it with
 * in-memory state + fixture images; TauriApi will call the Rust core later.
 */
export interface ApiAdapter {
  readonly mode: "mock" | "tauri";
  createProject(input: CreateProjectInput, options?: CallOptions): Promise<ProjectDetail>;
  listProjects(options?: CallOptions): Promise<ProjectSummary[]>;
  getProject(id: string, options?: CallOptions): Promise<ProjectDetail>;
  generateBoard(
    projectId: string,
    brief: BoardBrief,
    options?: GenerateOptions,
  ): Promise<GenerateResult>;
  pickAnchor(projectId: string, candidateId: string, options?: CallOptions): Promise<ProjectDetail>;
  addPage(
    projectId: string,
    input: { slug: string; brief: string },
    options?: CallOptions,
  ): Promise<ProjectDetail>;
  /** Amend a page's brief (core `page_update`); used by the regenerate drawer. */
  updatePage(
    projectId: string,
    slug: string,
    input: { brief: string },
    options?: CallOptions,
  ): Promise<ProjectDetail>;
  generatePage(
    projectId: string,
    slug: string,
    options?: GenerateOptions,
  ): Promise<GenerateResult>;
  pickPage(
    projectId: string,
    slug: string,
    candidateId: string,
    options?: CallOptions,
  ): Promise<ProjectDetail>;
  addComponent(
    projectId: string,
    input: { name: string; type: ComponentType; brief: string },
    options?: CallOptions,
  ): Promise<ProjectDetail>;
  /** Amend a component's brief (core `component_update`); used by the regenerate drawer. */
  updateComponent(
    projectId: string,
    name: string,
    input: { brief: string },
    options?: CallOptions,
  ): Promise<ProjectDetail>;
  generateComponent(
    projectId: string,
    name: string,
    options?: GenerateOptions,
  ): Promise<GenerateResult>;
  pickComponent(
    projectId: string,
    name: string,
    candidateId: string,
    options?: CallOptions,
  ): Promise<ProjectDetail>;
  exportProject(
    projectId: string,
    outDir: string,
    options?: GenerateOptions,
  ): Promise<ExportResult>;
  deleteArtifact(
    projectId: string,
    target: DeleteTarget,
    options?: CallOptions,
  ): Promise<ProjectDetail>;
  /**
   * Lineage of one stage image: the GenRecord batch that produced
   * `target`'s candidate. `null` when the image has no recorded lineage
   * (e.g. hand-placed) — an empty state, never an error.
   */
  getLineage(
    projectId: string,
    target: LineageTarget,
    options?: CallOptions,
  ): Promise<GenRecord | null>;
}
