import { useEffect, useState } from "react";
import {
  Boxes,
  ChevronDown,
  ChevronRight,
  LayoutDashboard,
  LayoutGrid,
  Plus,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type { TreeGroupId } from "@/state/studio";
import { treeNodeUnlocked, useStudio } from "@/state/studio";
import { useToast } from "@/state/toast";
import { AnchorBadge } from "@/components/card-actions";
import { ApiError } from "@/lib/api/types";
import { useConfirm } from "@/hooks/use-confirm";
import { requestTreeAdd } from "@/lib/events";
import { cn } from "@/lib/utils";

type ExpandableBranch = "pages" | "components";

type Expanded = { root: boolean; pages: boolean; components: boolean };

/**
 * Left-column directory tree (replaces the horizontal stepper): current
 * project root -> overview / pages / components groups -> page/component
 * leaves. Gating is migrated from the former `stepUnlockedMap` (PRD §3);
 * locked groups guide the user back to the overview instead of blocking.
 */
export function ProjectTree({ onNewProject }: { onNewProject: () => void }) {
  const { t } = useTranslation();
  const { state, setView, selectPage, selectComponent, deleteArtifact } = useStudio();
  const toast = useToast();
  const { ask, element: confirmElement } = useConfirm();
  const [expanded, setExpanded] = useState<Expanded>({
    root: true,
    pages: true,
    components: true,
  });
  const project = state.project;
  const unlocked = treeNodeUnlocked(project);

  // Keep the active view's branch visible (e.g. addPage → page:<slug>).
  useEffect(() => {
    if (state.view.startsWith("page:")) {
      setExpanded((prev) => ({ ...prev, root: true, pages: true }));
    } else if (state.view.startsWith("component:")) {
      setExpanded((prev) => ({ ...prev, root: true, components: true }));
    }
  }, [state.view]);

  if (!project) {
    return (
      <div className="flex flex-col gap-2 p-3" data-testid="project-tree">
        <button
          type="button"
          data-testid="tree-empty-create"
          onClick={onNewProject}
          className="flex items-center justify-center gap-1.5 rounded-md border border-dashed px-2 py-3 text-xs text-muted-foreground transition-colors duration-150 ease-out hover:border-primary/60 hover:text-foreground"
        >
          <Plus className="size-3.5" />
          {t("tree.emptyCta")}
        </button>
        <p className="text-[11px] leading-relaxed text-muted-foreground">
          {t("tree.emptyHint")}
        </p>
      </div>
    );
  }

  /** Locked groups keep the former stepper gating semantics: guide, don't block. */
  const guardGroup = (group: TreeGroupId): boolean => {
    if (unlocked[group]) return true;
    if (group === "pages") {
      toast.error(
        new ApiError(
          "ANCHOR_REQUIRED",
          t("errors.ANCHOR_REQUIRED.message"),
          t("errors.ANCHOR_REQUIRED.hint"),
        ),
      );
    } else if (group === "components") {
      toast.info(t("tree.lockedComponent"));
    }
    return false;
  };

  const openGroup = (group: TreeGroupId) => {
    if (!guardGroup(group)) return;
    setExpanded((prev) => ({ ...prev, root: true, [group]: true }));
    setView(group);
  };

  const toggleBranch = (branch: ExpandableBranch) => {
    if (!guardGroup(branch)) return;
    setExpanded((prev) => ({ ...prev, [branch]: !prev[branch] }));
  };

  const openPage = (slug: string) => {
    selectPage(slug);
    setView(`page:${slug}`);
  };

  const openComponent = (name: string) => {
    selectComponent(name);
    setView(`component:${name}`);
  };

  const removePage = (slug: string) => {
    ask({
      title: t("confirm.deleteItem.title"),
      description: t("confirm.deleteItem.desc", { name: slug }),
      onConfirm: () => void deleteArtifact({ kind: "page", slug }),
    });
  };

  const removeComponent = (name: string) => {
    ask({
      title: t("confirm.deleteItem.title"),
      description: t("confirm.deleteItem.desc", { name }),
      onConfirm: () => void deleteArtifact({ kind: "component", name }),
    });
  };

  const branchRow = (
    branch: ExpandableBranch | "overview",
    label: string,
    icon: React.ReactNode,
    count: number,
  ) => {
    const selected = state.view === branch;
    const locked = !unlocked[branch];
    const isLeaf = branch === "overview";
    const open = isLeaf ? false : expanded[branch];
    return (
      <div
        key={branch}
        className="group/row relative flex items-center"
        data-testid={`tree-group-${branch}`}
        data-state={locked ? "locked" : selected ? "current" : "default"}
      >
        {isLeaf ? (
          <span aria-hidden="true" className="h-6 w-5 shrink-0" />
        ) : (
          <button
            type="button"
            data-testid={`tree-group-toggle-${branch}`}
            aria-expanded={locked ? undefined : open}
            aria-disabled={locked || undefined}
            title={locked ? t("tree.lockedPage") : open ? t("tree.collapse") : t("tree.expand")}
            onClick={() => toggleBranch(branch)}
            className={cn(
              "flex h-6 w-5 shrink-0 items-center justify-center rounded-sm transition-colors duration-150 ease-out",
              locked ? "cursor-default" : "hover:bg-muted",
            )}
          >
            {open && !locked ? (
              <ChevronDown className="size-3.5 text-muted-foreground" />
            ) : (
              <ChevronRight className="size-3.5 text-muted-foreground" />
            )}
          </button>
        )}
        <button
          type="button"
          data-testid={`tree-group-open-${branch}`}
          aria-disabled={locked || undefined}
          aria-current={selected ? "true" : undefined}
          onClick={() => openGroup(branch)}
          className={cn(
            "flex min-w-0 flex-1 items-center gap-1.5 rounded-md border border-transparent px-1.5 py-1 text-left transition-colors duration-150 ease-out",
            selected && !locked && "border-primary/60 bg-primary/10 font-medium",
            locked
              ? "cursor-default text-muted-foreground opacity-60"
              : "hover:bg-muted/60",
          )}
        >
          {icon}
          <span className="truncate">{label}</span>
          <span className="ml-auto shrink-0 font-mono text-[10px] text-muted-foreground">
            {count}
          </span>
        </button>
        {!isLeaf && !locked && (
          <button
            type="button"
            data-testid={`tree-add-${branch}`}
            aria-label={t(branch === "pages" ? "tree.addPage" : "tree.addComponent")}
            title={t(branch === "pages" ? "tree.addPage" : "tree.addComponent")}
            onClick={() => {
              setExpanded((prev) => ({ ...prev, [branch]: true }));
              setView(branch);
              requestTreeAdd(branch === "pages" ? "page" : "component");
            }}
            className="ml-0.5 flex size-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground opacity-0 transition-opacity duration-150 ease-out hover:bg-muted hover:text-foreground focus-visible:opacity-100 group-hover/row:opacity-100"
          >
            <Plus className="size-3.5" />
          </button>
        )}
      </div>
    );
  };

  const leafRow = (
    kind: "page" | "component",
    id: string,
    picked: boolean,
    candidateCount: number,
  ) => {
    const target = kind === "page" ? `page:${id}` : `component:${id}`;
    const selected = state.view === target;
    return (
      <div key={id} className="group/leaf relative flex items-center">
        <button
          type="button"
          data-testid={`tree-${kind}-${id}`}
          aria-current={selected ? "true" : undefined}
          data-state={selected ? "current" : "default"}
          onClick={() => (kind === "page" ? openPage(id) : openComponent(id))}
          className={cn(
            "flex min-w-0 flex-1 items-center gap-1.5 rounded-md border border-transparent px-1.5 py-1 text-left font-mono text-[11px] transition-colors duration-150 ease-out",
            selected
              ? "border-primary/60 bg-primary/10 font-medium text-foreground"
              : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
          )}
        >
          <span
            aria-hidden="true"
            title={picked ? t("gallery.flow.picked") : undefined}
            className={cn(
              "size-1.5 shrink-0 rounded-full",
              picked && "bg-primary",
              !picked && candidateCount > 0 && "border border-primary/70",
              candidateCount === 0 && !picked && "border border-border",
            )}
          />
          <span className="truncate">{id}</span>
        </button>
        <button
          type="button"
          data-testid={`tree-${kind}-delete-${id}`}
          aria-label={t("common.delete")}
          title={t("common.delete")}
          onClick={() => (kind === "page" ? removePage(id) : removeComponent(id))}
          className="ml-0.5 flex size-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground opacity-0 transition-opacity duration-150 ease-out hover:bg-muted hover:text-destructive focus-visible:opacity-100 group-hover/leaf:opacity-100"
        >
          <Trash2 className="size-3.5" />
        </button>
      </div>
    );
  };

  return (
    <nav
      data-testid="project-tree"
      aria-label={t("tree.title")}
      className="flex min-h-0 flex-1 flex-col gap-0.5 p-2 pt-1"
    >
      {confirmElement}

      <div
        className="group/root flex items-center"
        data-testid="tree-root"
        data-expanded={expanded.root}
      >
        <button
          type="button"
          data-testid="tree-root-toggle"
          aria-expanded={expanded.root}
          title={expanded.root ? t("tree.collapse") : t("tree.expand")}
          onClick={() => setExpanded((prev) => ({ ...prev, root: !prev.root }))}
          className="flex h-6 w-5 shrink-0 items-center justify-center rounded-sm transition-colors duration-150 ease-out hover:bg-muted"
        >
          {expanded.root ? (
            <ChevronDown className="size-3.5 text-muted-foreground" />
          ) : (
            <ChevronRight className="size-3.5 text-muted-foreground" />
          )}
        </button>
        <div className="flex min-w-0 flex-1 items-center gap-1.5 rounded-md px-1.5 py-1">
          <span className="truncate text-xs font-semibold text-foreground">
            {project.name}
          </span>
          {project.anchor && <AnchorBadge className="scale-90" />}
        </div>
      </div>

      {expanded.root && (
        <div className="ml-3 flex flex-col gap-0.5 border-l pl-1.5">
          {branchRow(
            "overview",
            t("tree.overview"),
            <LayoutDashboard className="size-3.5 shrink-0 text-muted-foreground" />,
            project.boardCandidates.length,
          )}
          {branchRow(
            "pages",
            t("tree.pages"),
            <LayoutGrid className="size-3.5 shrink-0 text-muted-foreground" />,
            project.pages.length,
          )}
          {expanded.pages &&
            unlocked.pages &&
            project.pages.map((page) =>
              leafRow("page", page.slug, page.current !== null, page.candidates.length),
            )}
          {branchRow(
            "components",
            t("tree.components"),
            <Boxes className="size-3.5 shrink-0 text-muted-foreground" />,
            project.components.length,
          )}
          {expanded.components &&
            unlocked.components &&
            project.components.map((component) =>
              leafRow(
                "component",
                component.name,
                component.current !== null,
                component.candidates.length,
              ),
            )}
        </div>
      )}
    </nav>
  );
}
