import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";

import { createApi } from "@/lib/api";
import type {
  ApiAdapter,
  BoardBrief,
  ComponentType,
  CreateProjectInput,
  DeleteTarget,
  ExportResult,
  ProjectDetail,
  ProjectSummary,
} from "@/lib/api/types";
import { DEFAULT_QUALITY, type QualityLevel } from "@/lib/form-schema";
import { useToast } from "@/state/toast";

export type JobKind = "board" | "page" | "component" | "export";

/**
 * Tree navigation target (replaces the former horizontal StepId):
 * - overview / pages / components are the three fixed group nodes;
 * - `page:<slug>` / `component:<name>` select a leaf under its group.
 */
export type TreeViewId =
  | "overview"
  | "pages"
  | "components"
  | `page:${string}`
  | `component:${string}`;

/** Fixed group node ids (the tree's static level under the project root). */
export type TreeGroupId = "overview" | "pages" | "components";

/**
 * Whether a tree group may be opened: migrated from the former
 * `stepUnlockedMap` gating (PRD §3) — same invariants, new surface.
 */
export function treeNodeUnlocked(
  project: ProjectDetail | null,
): Record<TreeGroupId, boolean> {
  return {
    overview: project !== null,
    pages: project?.anchor != null,
    components:
      project?.anchor != null && project.pages.some((page) => page.current !== null),
  };
}

export interface ActiveJob {
  kind: JobKind;
  /** 0..1 */
  progress: number;
  startedAt: number;
  /** page slug / component name the job is for */
  target?: string;
}

export interface StudioState {
  projects: ProjectSummary[];
  project: ProjectDetail | null;
  projectsLoading: boolean;
  view: TreeViewId;
  selectedPage: string | null;
  selectedComponent: string | null;
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
  const [view, setView] = useState<TreeViewId>("overview");
  const [selectedPage, setSelectedPage] = useState<string | null>(null);
  const [selectedComponent, setSelectedComponent] = useState<string | null>(null);
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
    setSelectedPage(null);
    setSelectedComponent(null);
    // Auto-step equivalent: a fresh/switched project lands on the overview.
    setView("overview");
  }, []);

  const selectProject = useCallback(async (id: string) => {
    try {
      const detail = await api.getProject(id);
      activateProject(detail);
    } catch (error) {
      toast.error(error);
    }
  }, [activateProject, api, toast]);

  const createProject = useCallback(
    async (input: CreateProjectInput) => {
      try {
        const detail = await api.createProject(input);
        activateProject(detail);
        void refreshProjects();
        toast.success(t("toast.success.project"));
      } catch (error) {
        toast.error(error);
      }
    },
    [activateProject, api, refreshProjects, t, toast],
  );

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
        // No forced jump (tree era): the success toast invites the user to
        // start page design; the pages group unlocks in the tree.
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
        setSelectedPage(slug);
        setView(`page:${slug}`);
        void refreshProjects();
        toast.success(t("toast.success.pageAdded", { slug }));
      } catch (error) {
        toast.error(error);
      }
    },
    [api, project, refreshProjects, t, toast],
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
        setSelectedComponent(name);
        setView(`component:${name}`);
        void refreshProjects();
        toast.success(t("toast.success.componentAdded", { name }));
      } catch (error) {
        toast.error(error);
      }
    },
    [api, project, refreshProjects, t, toast],
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
        if (target.kind === "page") {
          setSelectedPage((prev) => (prev === target.slug ? null : prev));
          setView((prev) => (prev === `page:${target.slug}` ? "pages" : prev));
        }
        if (target.kind === "component") {
          setSelectedComponent((prev) => (prev === target.name ? null : prev));
          setView((prev) => (prev === `component:${target.name}` ? "components" : prev));
        }
        toast.success(t("toast.success.deleted"));
      } catch (error) {
        toast.error(error);
      }
    },
    [api, project, refreshProjects, t, toast],
  );

  const state = useMemo<StudioState>(
    () => ({
      projects,
      project,
      projectsLoading,
      view,
      selectedPage,
      selectedComponent,
      job,
      lastExport,
    }),
    [projects, project, projectsLoading, view, selectedPage, selectedComponent, job, lastExport],
  );

  const value = useMemo<StudioApi>(
    () => ({
      state,
      apiMode: api.mode,
      setView,
      selectPage: (slug: string | null) => setSelectedPage(slug),
      selectComponent: (name: string | null) => setSelectedComponent(name),
      cancelJob,
      createProject,
      selectProject,
      generateBoard,
      pickAnchor,
      addPage,
      generatePage,
      pickPage,
      addComponent,
      generateComponent,
      pickComponent,
      exportProject,
      deleteArtifact,
    }),
    [
      state,
      api.mode,
      cancelJob,
      createProject,
      selectProject,
      generateBoard,
      pickAnchor,
      addPage,
      generatePage,
      pickPage,
      addComponent,
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
  selectPage: (slug: string | null) => void;
  selectComponent: (name: string | null) => void;
  cancelJob: () => void;
  createProject: (input: CreateProjectInput) => Promise<void>;
  selectProject: (id: string) => Promise<void>;
  generateBoard: (brief: BoardBrief, count: number, quality?: QualityLevel) => Promise<void>;
  pickAnchor: (candidateId: string) => Promise<void>;
  addPage: (slug: string, brief: string) => Promise<void>;
  generatePage: (slug: string, count: number, quality?: QualityLevel) => Promise<void>;
  pickPage: (slug: string, candidateId: string) => Promise<void>;
  addComponent: (name: string, type: ComponentType, brief: string) => Promise<void>;
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
