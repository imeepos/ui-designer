import {
  ApiError,
  type ApiAdapter,
  type BoardBrief,
  type CallOptions,
  type CreateProjectInput,
  type DeleteTarget,
  type ExportResult,
  type GenerateOptions,
  type GenerateResult,
  type ProjectDetail,
  type ProjectSummary,
} from "@/lib/api/types";

function notImplemented(command: string): ApiError {
  return new ApiError(
    "NOT_IMPLEMENTED",
    `Tauri command not wired in this phase: ${command}`,
    "The desktop shell calls rudder-core here in a later phase; the mock adapter covers the flow for now",
  );
}

/**
 * Tauri-backed adapter stub. Enabled when `window.__TAURI__` is present.
 * Phase 3 (mock-first): every method rejects with NOT_IMPLEMENTED so the UI
 * contract is exercised without touching the Rust core.
 */
export class TauriApi implements ApiAdapter {
  readonly mode = "tauri" as const;

  async createProject(_input: CreateProjectInput, _options?: CallOptions): Promise<ProjectDetail> {
    throw notImplemented("create_project");
  }

  async listProjects(_options?: CallOptions): Promise<ProjectSummary[]> {
    throw notImplemented("list_projects");
  }

  async getProject(_id: string, _options?: CallOptions): Promise<ProjectDetail> {
    throw notImplemented("get_project");
  }

  async generateBoard(
    _projectId: string,
    _brief: BoardBrief,
    _options?: GenerateOptions,
  ): Promise<GenerateResult> {
    throw notImplemented("generate_board");
  }

  async pickAnchor(
    _projectId: string,
    _candidateId: string,
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    throw notImplemented("pick_anchor");
  }

  async addPage(
    _projectId: string,
    _input: { slug: string; brief: string },
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    throw notImplemented("add_page");
  }

  async generatePage(
    _projectId: string,
    _slug: string,
    _options?: GenerateOptions,
  ): Promise<GenerateResult> {
    throw notImplemented("generate_page");
  }

  async pickPage(
    _projectId: string,
    _slug: string,
    _candidateId: string,
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    throw notImplemented("pick_page");
  }

  async addComponent(
    _projectId: string,
    _input: { name: string; type: string; brief: string },
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    throw notImplemented("add_component");
  }

  async generateComponent(
    _projectId: string,
    _name: string,
    _options?: GenerateOptions,
  ): Promise<GenerateResult> {
    throw notImplemented("generate_component");
  }

  async pickComponent(
    _projectId: string,
    _name: string,
    _candidateId: string,
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    throw notImplemented("pick_component");
  }

  async exportProject(
    _projectId: string,
    _outDir: string,
    _options?: GenerateOptions,
  ): Promise<ExportResult> {
    throw notImplemented("export_project");
  }

  async deleteArtifact(
    _projectId: string,
    _target: DeleteTarget,
    _options?: CallOptions,
  ): Promise<ProjectDetail> {
    throw notImplemented("delete_artifact");
  }
}
