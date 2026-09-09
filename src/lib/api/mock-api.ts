import {
  ApiError,
  type ApiAdapter,
  type BoardBrief,
  type CallOptions,
  type Candidate,
  type ComponentItem,
  type ComponentType,
  type CreateProjectInput,
  type DeleteTarget,
  type ExportResult,
  type GenerateOptions,
  type GenerateResult,
  type PageItem,
  type ProjectDetail,
  type ProjectSummary,
} from "@/lib/api/types";
import { BOARD_FIXTURE, PAGE_FIXTURE, variantFilter } from "@/lib/api/mock/fixtures";
import { isValidSlug } from "@/lib/validate";
import { validateCanvasSize } from "@/lib/size";

// Real generations take 30s..3min; the mock compresses that to 2..5s so the
// full flow stays walkable, while the loading UI keeps the real expectation.
const MIN_DELAY_MS = 2_000;
const MAX_DELAY_MS = 5_000;
const PROGRESS_TICK_MS = 120;

type StorePage = PageItem;
type StoreComponent = ComponentItem;
type StoreProject = ProjectDetail;

function delay(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(new ApiError("CANCELLED", "Operation cancelled"));
      return;
    }
    const timer = setTimeout(() => {
      signal?.removeEventListener("abort", onAbort);
      resolve();
    }, ms);
    function onAbort() {
      clearTimeout(timer);
      reject(new ApiError("CANCELLED", "Operation cancelled"));
    }
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}

function randomDelay(range: { min: number; max: number }): number {
  return range.min + Math.random() * (range.max - range.min);
}

/** Simulated progress: 0 -> 0.9 over `ms`, then the caller finishes to 1. */
async function simulateWork(ms: number, options?: GenerateOptions): Promise<void> {
  const signal = options?.signal;
  const onProgress = options?.onProgress;
  if (!onProgress) {
    await delay(ms, signal);
    return;
  }
  const tick = Math.min(PROGRESS_TICK_MS, ms);
  const steps = Math.max(1, Math.floor(ms / tick));
  for (let i = 1; i <= steps; i += 1) {
    await delay(ms / steps, signal);
    onProgress(Math.min(0.9, (i / steps) * 0.9));
  }
}

function requireProject(store: Map<string, StoreProject>, id: string): StoreProject {
  const project = store.get(id);
  if (!project) {
    throw new ApiError("NOT_FOUND", `Project not found: ${id}`);
  }
  return project;
}

function requireAnchor(project: StoreProject): void {
  if (!project.anchor) {
    throw new ApiError(
      "ANCHOR_REQUIRED",
      "No anchor board selected yet",
      "Pick a board candidate as the anchor before generating pages or components",
    );
  }
}

function makeCandidates(count: number, fixture: string, seedBase: number): Candidate[] {
  const now = Date.now();
  return Array.from({ length: count }, (_, index) => ({
    id: `c-${seedBase.toString(36)}-${index + 1}`,
    url: fixture,
    createdAt: now + index,
    seed: seedBase + index,
    filter: variantFilter(index),
  }));
}

function nextSeed(): number {
  seedCounter += 1;
  return seedCounter;
}

let seedCounter = 100;
let idCounter = 0;

/**
 * In-memory adapter with fixture images. Public for tests and storybook-ish
 * previews; the UI only talks to the ApiAdapter interface.
 *
 * Demo hook: a brief containing the literal token "/fail" simulates a remote
 * API error, so the error toast path can be exercised without a real failure.
 */
export class MockApi implements ApiAdapter {
  readonly mode = "mock" as const;
  private store = new Map<string, StoreProject>();
  private readonly delayRange: { min: number; max: number };

  constructor(delayRange: { min: number; max: number } = { min: MIN_DELAY_MS, max: MAX_DELAY_MS }) {
    this.delayRange = delayRange;
  }

  private nextId(prefix: string): string {
    idCounter += 1;
    return `${prefix}-${Date.now().toString(36)}-${idCounter}`;
  }

  private assertNotAborted(signal?: AbortSignal): void {
    if (signal?.aborted) {
      throw new ApiError("CANCELLED", "Operation cancelled");
    }
  }

  async createProject(
    input: CreateProjectInput,
    options?: CallOptions,
  ): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    const name = input.name.trim();
    if (!name) {
      throw new ApiError("VALIDATION_ERROR", "Project name is required");
    }
    const sizeCheck = validateCanvasSize(input.size.preset, String(input.size.w), String(input.size.h));
    if (!sizeCheck.ok) {
      throw new ApiError("VALIDATION_ERROR", `Invalid canvas size (${sizeCheck.code})`, undefined, sizeCheck.params);
    }
    const project: StoreProject = {
      id: this.nextId("p"),
      name,
      size: { ...input.size },
      brandBrief: input.brandBrief.trim(),
      styleBrief: (input.styleBrief ?? "").trim(),
      anchor: null,
      boardCandidates: [],
      pages: [],
      components: [],
      createdAt: Date.now(),
    };
    this.store.set(project.id, project);
    return clone(project);
  }

  async listProjects(options?: CallOptions): Promise<ProjectSummary[]> {
    this.assertNotAborted(options?.signal);
    return [...this.store.values()]
      .sort((a, b) => b.createdAt - a.createdAt)
      .map((project) => toSummary(project));
  }

  async getProject(id: string, options?: CallOptions): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    return clone(requireProject(this.store, id));
  }

  async generateBoard(
    projectId: string,
    brief: BoardBrief,
    options?: GenerateOptions,
  ): Promise<GenerateResult> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    if (!brief.brandKeywords.trim()) {
      throw new ApiError("VALIDATION_ERROR", "Brand keywords are required");
    }
    project.brandBrief = brief.brandKeywords.trim();
    project.styleBrief = [brief.colorDirection, brief.fontMood, brief.radiusDensity]
      .map((part) => part.trim())
      .filter(Boolean)
      .join(" / ");
    if (brief.reference.trim()) {
      project.styleBrief = `${project.styleBrief} | ${brief.reference.trim()}`;
    }
    if (brief.brandKeywords.includes("/fail")) {
      await delay(800, options?.signal);
      throw new ApiError(
        "API_ERROR",
        "Simulated upstream image API failure",
        "Remove the /fail token from the brief and retry",
      );
    }
    const count = clampCount(options?.count);
    await simulateWork(randomDelay(this.delayRange), options);
    const candidates = makeCandidates(count, BOARD_FIXTURE, nextSeed());
    project.boardCandidates.push(...candidates);
    return { candidates, project: clone(project) };
  }

  async pickAnchor(
    projectId: string,
    candidateId: string,
    options?: CallOptions,
  ): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    const candidate = project.boardCandidates.find((item) => item.id === candidateId);
    if (!candidate) {
      throw new ApiError("NOT_FOUND", `Board candidate not found: ${candidateId}`);
    }
    project.anchor = {
      candidateId: candidate.id,
      url: candidate.url,
      filter: candidate.filter,
      createdAt: Date.now(),
    };
    return clone(project);
  }

  async addPage(
    projectId: string,
    input: { slug: string; brief: string },
    options?: CallOptions,
  ): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    const slug = input.slug.trim();
    if (!isValidSlug(slug)) {
      throw new ApiError("VALIDATION_ERROR", `Invalid page slug: ${slug}`);
    }
    if (project.pages.some((page) => page.slug === slug)) {
      throw new ApiError("DUPLICATE_SLUG", `Page already exists: ${slug}`);
    }
    if (!input.brief.trim()) {
      throw new ApiError("VALIDATION_ERROR", "Page brief is required");
    }
    project.pages.push({
      slug,
      brief: input.brief.trim(),
      candidates: [],
      current: null,
      history: [],
      updatedAt: Date.now(),
    });
    return clone(project);
  }

  async updatePage(
    projectId: string,
    slug: string,
    input: { brief: string },
    options?: CallOptions,
  ): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    const page = requirePage(project, slug);
    if (!input.brief.trim()) {
      throw new ApiError("VALIDATION_ERROR", "Page brief is required");
    }
    page.brief = input.brief.trim();
    page.updatedAt = Date.now();
    return clone(project);
  }

  async generatePage(
    projectId: string,
    slug: string,
    options?: GenerateOptions,
  ): Promise<GenerateResult> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    requireAnchor(project);
    const page = requirePage(project, slug);
    if (page.brief.includes("/fail")) {
      await delay(800, options?.signal);
      throw new ApiError(
        "API_ERROR",
        "Simulated upstream image API failure",
        "Remove the /fail token from the brief and retry",
      );
    }
    const count = clampCount(options?.count);
    await simulateWork(randomDelay(this.delayRange), options);
    const candidates = makeCandidates(count, PAGE_FIXTURE, nextSeed());
    page.candidates.push(...candidates);
    page.updatedAt = Date.now();
    return { candidates, project: clone(project) };
  }

  async pickPage(
    projectId: string,
    slug: string,
    candidateId: string,
    options?: CallOptions,
  ): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    const page = requirePage(project, slug);
    const candidate = page.candidates.find((item) => item.id === candidateId);
    if (!candidate) {
      throw new ApiError("NOT_FOUND", `Page candidate not found: ${candidateId}`);
    }
    if (page.current) {
      page.history.push({ ts: Date.now(), url: page.current.url, filter: page.current.filter });
    }
    page.current = { url: candidate.url, candidateId: candidate.id, filter: candidate.filter };
    page.candidates = [];
    page.updatedAt = Date.now();
    return clone(project);
  }

  async addComponent(
    projectId: string,
    input: { name: string; type: ComponentType; brief: string },
    options?: CallOptions,
  ): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    const name = input.name.trim();
    if (!name) {
      throw new ApiError("VALIDATION_ERROR", "Component name is required");
    }
    if (project.components.some((component) => component.name === name)) {
      throw new ApiError("DUPLICATE_NAME", `Component already exists: ${name}`);
    }
    if (!input.brief.trim()) {
      throw new ApiError("VALIDATION_ERROR", "Component brief is required");
    }
    project.components.push({
      name,
      type: input.type,
      brief: input.brief.trim(),
      candidates: [],
      current: null,
      history: [],
      updatedAt: Date.now(),
    });
    return clone(project);
  }

  async updateComponent(
    projectId: string,
    name: string,
    input: { brief: string },
    options?: CallOptions,
  ): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    const component = requireComponent(project, name);
    if (!input.brief.trim()) {
      throw new ApiError("VALIDATION_ERROR", "Component brief is required");
    }
    component.brief = input.brief.trim();
    component.updatedAt = Date.now();
    return clone(project);
  }

  async generateComponent(
    projectId: string,
    name: string,
    options?: GenerateOptions,
  ): Promise<GenerateResult> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    requireAnchor(project);
    const component = requireComponent(project, name);
    if (component.brief.includes("/fail")) {
      await delay(800, options?.signal);
      throw new ApiError(
        "API_ERROR",
        "Simulated upstream image API failure",
        "Remove the /fail token from the brief and retry",
      );
    }
    const count = clampCount(options?.count);
    await simulateWork(randomDelay(this.delayRange), options);
    const candidates = makeCandidates(count, PAGE_FIXTURE, nextSeed());
    component.candidates.push(...candidates);
    component.updatedAt = Date.now();
    return { candidates, project: clone(project) };
  }

  async pickComponent(
    projectId: string,
    name: string,
    candidateId: string,
    options?: CallOptions,
  ): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    const component = requireComponent(project, name);
    const candidate = component.candidates.find((item) => item.id === candidateId);
    if (!candidate) {
      throw new ApiError("NOT_FOUND", `Component candidate not found: ${candidateId}`);
    }
    if (component.current) {
      component.history.push({
        ts: Date.now(),
        url: component.current.url,
        filter: component.current.filter,
      });
    }
    component.current = {
      url: candidate.url,
      candidateId: candidate.id,
      filter: candidate.filter,
    };
    component.candidates = [];
    component.updatedAt = Date.now();
    return clone(project);
  }

  async exportProject(
    projectId: string,
    outDir: string,
    options?: GenerateOptions,
  ): Promise<ExportResult> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    const dir = outDir.trim();
    if (!dir) {
      throw new ApiError("VALIDATION_ERROR", "Export directory is required");
    }
    const imageCount =
      (project.anchor ? 1 : project.boardCandidates.length) +
      project.pages.reduce((sum, page) => sum + (page.current ? 1 : 0), 0) +
      project.components.reduce((sum, component) => sum + (component.current ? 1 : 0), 0);
    if (imageCount === 0) {
      throw new ApiError(
        "EXPORT_FAILED",
        "Nothing to export yet",
        "Generate and pick at least one board, page or component image first",
      );
    }
    await simulateWork(randomDelay(this.delayRange) / 2, options);
    const files: ExportResult["files"] = [];
    const pushImage = (path: string) => files.push({ path, bytes: 900_000, kind: "image" });
    if (project.anchor) {
      pushImage("board/anchor.png");
    }
    for (const candidate of project.boardCandidates) {
      pushImage(`board/candidates/${candidate.id}.png`);
    }
    for (const page of project.pages) {
      if (page.current) pushImage(`pages/${page.slug}/current.png`);
      page.candidates.forEach((candidate, index) => {
        pushImage(`pages/${page.slug}/candidates/${pad(index + 1)}-${candidate.id}.png`);
      });
    }
    for (const component of project.components) {
      if (component.current) pushImage(`components/${component.name}/current.png`);
      component.candidates.forEach((candidate, index) => {
        pushImage(`components/${component.name}/candidates/${pad(index + 1)}-${candidate.id}.png`);
      });
    }
    files.push({ path: "manifest.json", bytes: 4_800, kind: "manifest" });
    files.push({ path: "PROMPTS.md", bytes: 2_600, kind: "prompts" });
    files.push({ path: "DESIGN.template.md", bytes: 1_900, kind: "template" });
    return { outDir: dir, files };
  }

  async deleteArtifact(
    projectId: string,
    target: DeleteTarget,
    options?: CallOptions,
  ): Promise<ProjectDetail> {
    this.assertNotAborted(options?.signal);
    const project = requireProject(this.store, projectId);
    switch (target.kind) {
      case "boardCandidate": {
        if (project.anchor?.candidateId === target.candidateId) {
          throw new ApiError(
            "ANCHOR_LOCKED",
            "The anchor board cannot be deleted",
            "Pick another candidate as anchor first",
          );
        }
        project.boardCandidates = project.boardCandidates.filter(
          (candidate) => candidate.id !== target.candidateId,
        );
        break;
      }
      case "page": {
        project.pages = project.pages.filter((page) => page.slug !== target.slug);
        break;
      }
      case "pageCurrent": {
        const page = requirePage(project, target.slug);
        if (!page.current) {
          throw new ApiError("NOT_FOUND", `Page has no current image: ${target.slug}`);
        }
        page.current = null;
        break;
      }
      case "pageHistory": {
        const page = requirePage(project, target.slug);
        page.history = page.history.filter((entry) => entry.ts !== target.ts);
        break;
      }
      case "component": {
        project.components = project.components.filter(
          (component) => component.name !== target.name,
        );
        break;
      }
      case "componentCurrent": {
        const component = requireComponent(project, target.name);
        if (!component.current) {
          throw new ApiError("NOT_FOUND", `Component has no current image: ${target.name}`);
        }
        component.current = null;
        break;
      }
      case "componentHistory": {
        const component = requireComponent(project, target.name);
        component.history = component.history.filter((entry) => entry.ts !== target.ts);
        break;
      }
    }
    return clone(project);
  }
}

function requirePage(project: StoreProject, slug: string): StorePage {
  const page = project.pages.find((item) => item.slug === slug);
  if (!page) {
    throw new ApiError("NOT_FOUND", `Page not found: ${slug}`);
  }
  return page;
}

function requireComponent(project: StoreProject, name: string): StoreComponent {
  const component = project.components.find((item) => item.name === name);
  if (!component) {
    throw new ApiError("NOT_FOUND", `Component not found: ${name}`);
  }
  return component;
}

function clampCount(count: number | undefined): number {
  const value = count ?? 2;
  return Math.max(1, Math.min(4, Math.floor(value)));
}

function pad(value: number): string {
  return value.toString().padStart(4, "0");
}

function toSummary(project: StoreProject): ProjectSummary {
  return {
    id: project.id,
    name: project.name,
    size: { ...project.size },
    createdAt: project.createdAt,
    hasAnchor: project.anchor !== null,
    pageCount: project.pages.length,
    componentCount: project.components.length,
  };
}

function clone<T>(value: T): T {
  return structuredClone(value);
}
