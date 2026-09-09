import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import type { ProjectDetail } from "@/lib/api/types";
import { createApi } from "@/lib/api";
import type {
  ApiAdapter,
  BoardBrief,
  ComponentType,
  CreateProjectInput,
  DeleteTarget,
  ExportResult,
  ProjectSummary,
} from "@/lib/api/types";
import { DEFAULT_QUALITY, type QualityLevel } from "@/lib/form-schema";
import { useToast } from "@/state/toast";

export type JobKind = "board" | "page" | "component" | "export";

export interface ActiveJob {
  kind: JobKind;
  /** 0..1 */
  progress: number;
  startedAt: number;
  /** page slug / component name the job is for */
  target?: string;
}

/**
 * Workspace selection: the fixed overview entry (anchor stage) or a
 * page/component leaf selected in the left menu.
 */
export type TreeViewId = "overview" | `page:${string}` | `component:${string}`;

/** Left-menu group ids; `unlocked` gates only the inline add actions. */
export type MenuGroupId = "overview" | "pages" | "components";

/**
 * Whether a menu group's add action may run: no anchor -> guide back to
 * overview (PRD §3 gating, former `stepUnlockedMap` invariants).
 */
export function menuUnlocked(
  project: ProjectDetail | null,
): Record<MenuGroupId, boolean> {
  return {
    overview: project !== null,
    pages: project?.anchor != null,
    components: project?.anchor != null,
  };
}

export interface StudioState {
  projects: ProjectSummary[];
  project: ProjectDetail | null;
  projectsLoading: boolean;
  /** Home (project list) vs workspace routing. */
  entered: boolean;
  view: TreeViewId;
  job: ActiveJob | null;
  lastExport: ExportResult | null;
}

const StudioContext = createContext<StudioApi | null>(null);

export function StudioProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const toast = useToast();
  const apiRef = useRef<ApiAdapter | null>(null);
  if (apiRef.current === null) {
    apiRef.current = createApi();
  }
  const api = apiRef.current;

  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [projectsLoading, setProjectsLoading] = useState(true);
  const [project, setProject] = useState<ProjectDetail | null>(null);
  const [entered, setEntered] = useState(false);
  const [view, setView] = useState<TreeViewId>("overview");
  const [job, setJob] = useState<ActiveJob | null>(null);
  const [lastExport, setLastExport] = useState<ExportResult | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .listProjects()
      .then((list) => {
        if (!cancelled) setProjects(list);
      })
      .catch((error) => {
        if (!cancelled) toast.error(error);
      })
      .finally(() => {
        if (!cancelled) setProjectsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [api, toast]);

  /** Wraps a generation-style call with job state, progress and cancellation. */
  const runJob = useCallback(
    async <T,>(
      kind: JobKind,
      target: string | undefined,
      run: (signal: AbortSignal, onProgress: (p: number) => void) => Promise<T>,
    ): Promise<T | null> => {
      if (abortRef.current) {
        abortRef.current.abort();
      }
      const controller = new AbortController();
      abortRef.current = controller;
      setJob({ kind, target, progress: 0, startedAt: Date.now() });
      try {
        const result = await run(controller.signal, (progress) => {
          setJob((prev) => (prev && prev.kind === kind ? { ...prev, progress } : prev));
        });
        setJob((prev) => (prev && prev.kind === kind ? { ...prev, progress: 1 } : prev));
        return result;
      } catch (error) {
        toast.error(error);
        return null;
      } finally {
        if (abortRef.current === controller) {
          abortRef.current = null;
          setJob(null);
        }
      }
    },
    [toast],
  );

  const cancelJob = useCallback(() => {
    abortRef.current?.abort();
  }, []);

  const refreshProjects = useCallback(async () => {
    try {
      setProjects(await api.listProjects());
    } catch (error) {
      toast.error(error);
    }
  }, [api, toast]);

  const activateProject = useCallback((detail: ProjectDetail) => {
    setProject(detail);
    setView("overview");
  }, []);

  /** Open a project from the home list -> enter its workspace. */
  const selectProject = useCallback(
    async (id: string) => {
      try {
        const detail = await api.getProject(id);
        activateProject(detail);
        setEntered(true);
      } catch (error) {
        toast.error(error);
      }
    },
    [activateProject, api, toast],
  );

  /**
   * Wizard step 1: create the project record but stay in the wizard; the
   * workspace is only entered via `enterWorkspace` (step 3 save).
   */
  const createProject = useCallback(
    async (input: CreateProjectInput): Promise<ProjectDetail | null> => {
      try {
        const detail = await api.createProject(input);
        activateProject(detail);
        void refreshProjects();
        toast.success(t("toast.success.project"));
        return detail;
      } catch (error) {
        toast.error(error);
        return null;
      }
    },
    [activateProject, api, refreshProjects, t, toast],
  );

  /** Wizard step 3: save -> leave the wizard into the project workspace. */
  const enterWorkspace = useCallback(() => {
    setEntered(true);
  }, []);

  /** Back from the workspace to the home project list. */
  const closeProject = useCallback(() => {
    setProject(null);
    setEntered(false);
    setView("overview");
  }, []);

  const generateBoard = useCallback(
    async (brief: BoardBrief, count: number, quality: QualityLevel = DEFAULT_QUALITY) => {
      if (!project) return;
      const result = await runJob("board", undefined, (signal, onProgress) =>
        api.generateBoard(project.id, brief, { count, quality, signal, onProgress }),
      );
      if (!result) return;
      setProject(result.project);
      void refreshProjects();
      toast.success(t("toast.success.board", { count: result.candidates.length }));
    },
    [api, project, refreshProjects, runJob, t, toast],
  );

  const pickAnchor = useCallback(
    async (candidateId: string) => {
      if (!project) return;
      try {
        const detail = await api.pickAnchor(project.id, candidateId);
        setProject(detail);
        void refreshProjects();
        // Guidance only: the pages group unlocks; no forced navigation.
        toast.success(t("toast.success.anchor"));
      } catch (error) {
        toast.error(error);
      }
    },
    [api, project, refreshProjects, t, toast],
  );

  const addPage = useCallback(
    async (slug: string, brief: string) => {
      if (!project) return;
      try {
        const detail = await api.addPage(project.id, { slug, brief });
        setProject(detail);
        setView(`page:${slug}`);
        void refreshProjects();
        toast.success(t("toast.success.pageAdded", { slug }));
      } catch (error) {
        toast.error(error);
      }
    },
    [api, project, refreshProjects, t, toast],
  );

  const updatePageBrief = useCallback(
    async (slug: string, brief: string) => {
      if (!project) return false;
      try {
        const detail = await api.updatePage(project.id, slug, { brief });
        setProject(detail);
        return true;
      } catch (error) {
        toast.error(error);
        return false;
      }
    },
    [api, project, toast],
  );

  const generatePage = useCallback(
    async (slug: string, count: number, quality: QualityLevel = DEFAULT_QUALITY) => {
      if (!project) return;
      const result = await runJob("page", slug, (signal, onProgress) =>
        api.generatePage(project.id, slug, { count, quality, signal, onProgress }),
      );
      if (!result) return;
      setProject(result.project);
      toast.success(t("toast.success.page", { slug, count: result.candidates.length }));
    },
    [api, project, runJob, t, toast],
  );

  const pickPage = useCallback(
    async (slug: string, candidateId: string) => {
      if (!project) return;
      try {
        const detail = await api.pickPage(project.id, slug, candidateId);
        setProject(detail);
        void refreshProjects();
        toast.success(t("toast.success.pagePicked", { slug }));
      } catch (error) {
        toast.error(error);
      }
    },
    [api, project, refreshProjects, t, toast],
  );

  const addComponent = useCallback(
    async (name: string, type: ComponentType, brief: string) => {
      if (!project) return;
      try {
        const detail = await api.addComponent(project.id, { name, type, brief });
        setProject(detail);
        setView(`component:${name}`);
        void refreshProjects();
        toast.success(t("toast.success.componentAdded", { name }));
      } catch (error) {
        toast.error(error);
      }
    },
    [api, project, refreshProjects, t, toast],
  );

  const updateComponentBrief = useCallback(
    async (name: string, brief: string) => {
      if (!project) return false;
      try {
        const detail = await api.updateComponent(project.id, name, { brief });
        setProject(detail);
        return true;
      } catch (error) {
        toast.error(error);
        return false;
      }
    },
    [api, project, toast],
  );

  const generateComponent = useCallback(
    async (name: string, count: number, quality: QualityLevel = DEFAULT_QUALITY) => {
      if (!project) return;
      const result = await runJob("component", name, (signal, onProgress) =>
        api.generateComponent(project.id, name, { count, quality, signal, onProgress }),
      );
      if (!result) return;
      setProject(result.project);
      toast.success(t("toast.success.component", { name, count: result.candidates.length }));
    },
    [api, project, runJob, t, toast],
  );

  const pickComponent = useCallback(
    async (name: string, candidateId: string) => {
      if (!project) return;
      try {
        const detail = await api.pickComponent(project.id, name, candidateId);
        setProject(detail);
        void refreshProjects();
        toast.success(t("toast.success.componentPicked", { name }));
      } catch (error) {
        toast.error(error);
      }
    },
    [api, project, refreshProjects, t, toast],
  );

  const exportProject = useCallback(
    async (outDir: string) => {
      if (!project) return;
      const result = await runJob("export", undefined, (signal, onProgress) =>
        api.exportProject(project.id, outDir, { signal, onProgress }),
      );
      if (!result) return;
      setLastExport(result);
      toast.success(t("toast.success.export", { count: result.files.length }));
    },
    [api, project, runJob, t, toast],
  );

  const deleteArtifact = useCallback(
    async (target: DeleteTarget) => {
      if (!project) return;
      try {
        const detail = await api.deleteArtifact(project.id, target);
        setProject(detail);
        void refreshProjects();
        if (target.kind === "page" && view === `page:${target.slug}`) {
          setView("overview");
        }
        if (target.kind === "component" && view === `component:${target.name}`) {
          setView("overview");
        }
        toast.success(t("toast.success.deleted"));
      } catch (error) {
        toast.error(error);
      }
    },
    [api, project, refreshProjects, t, toast, view],
  );

  const state = useMemo<StudioState>(
    () => ({
      projects,
      project,
      projectsLoading,
      entered,
      view,
      job,
      lastExport,
    }),
    [projects, project, projectsLoading, entered, view, job, lastExport],
  );

  const value = useMemo<StudioApi>(
    () => ({
      state,
      apiMode: api.mode,
      setView,
      enterWorkspace,
      closeProject,
      cancelJob,
      createProject,
      selectProject,
      generateBoard,
      pickAnchor,
      addPage,
      updatePageBrief,
      generatePage,
      pickPage,
      addComponent,
      updateComponentBrief,
      generateComponent,
      pickComponent,
      exportProject,
      deleteArtifact,
    }),
    [
      state,
      api.mode,
      cancelJob,
      closeProject,
      createProject,
      enterWorkspace,
      selectProject,
      generateBoard,
      pickAnchor,
      addPage,
      updatePageBrief,
      generatePage,
      pickPage,
      addComponent,
      updateComponentBrief,
      generateComponent,
      pickComponent,
      exportProject,
      deleteArtifact,
    ],
  );

  return <StudioContext.Provider value={value}>{children}</StudioContext.Provider>;
}

export interface StudioApi {
  state: StudioState;
  apiMode: "mock" | "tauri";
  setView: (view: TreeViewId) => void;
  enterWorkspace: () => void;
  closeProject: () => void;
  cancelJob: () => void;
  /** Wizard step 1: create without entering; returns the detail or null. */
  createProject: (input: CreateProjectInput) => Promise<ProjectDetail | null>;
  /** Home list -> open a project into the workspace. */
  selectProject: (id: string) => Promise<void>;
  generateBoard: (brief: BoardBrief, count: number, quality?: QualityLevel) => Promise<void>;
  pickAnchor: (candidateId: string) => Promise<void>;
  addPage: (slug: string, brief: string) => Promise<void>;
  /** Persist an amended brief before regenerating (core update ops). */
  updatePageBrief: (slug: string, brief: string) => Promise<boolean>;
  generatePage: (slug: string, count: number, quality?: QualityLevel) => Promise<void>;
  pickPage: (slug: string, candidateId: string) => Promise<void>;
  addComponent: (name: string, type: ComponentType, brief: string) => Promise<void>;
  updateComponentBrief: (name: string, brief: string) => Promise<boolean>;
  generateComponent: (name: string, count: number, quality?: QualityLevel) => Promise<void>;
  pickComponent: (name: string, candidateId: string) => Promise<void>;
  exportProject: (outDir: string) => Promise<void>;
  deleteArtifact: (target: DeleteTarget) => Promise<void>;
}

export function useStudio(): StudioApi {
  const ctx = useContext(StudioContext);
  if (!ctx) {
    throw new Error("useStudio must be used within StudioProvider");
  }
  return ctx;
}
