import { useState } from "react";
import {
  Boxes,
  Compass,
  LayoutGrid,
  Plus,
  Search,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import type { MenuGroupId } from "@/state/studio";
import { menuUnlocked, useStudio } from "@/state/studio";
import { useToast } from "@/state/toast";
import { AnchorBadge } from "@/components/anchor-badge";
import { ApiError } from "@/lib/api/types";
import { useConfirm } from "@/hooks/use-confirm";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

/**
 * Workspace left menu: search, fixed overview entry, then pages/components
 * groups with inline add actions and their entries (PRD v0.2 IA).
 */
export function LeftMenu({
  onAddPage,
  onAddComponent,
}: {
  onAddPage: () => void;
  onAddComponent: () => void;
}) {
  const { t } = useTranslation();
  const { state, setView, deleteArtifact } = useStudio();
  const toast = useToast();
  const { ask, element: confirmElement } = useConfirm();
  const [keyword, setKeyword] = useState("");

  const project = state.project;
  if (!project) return null;

  const unlocked = menuUnlocked(project);
  const query = keyword.trim().toLowerCase();
  const pages = project.pages.filter((page) => page.slug.toLowerCase().includes(query));
  const components = project.components.filter((component) =>
    component.name.toLowerCase().includes(query),
  );

  /** Gating keeps the former invariant: no anchor -> guide back to overview. */
  const guardAdd = (group: MenuGroupId): boolean => {
    if (unlocked[group]) return true;
    toast.error(
      new ApiError(
        "ANCHOR_REQUIRED",
        t("errors.ANCHOR_REQUIRED.message"),
        t("errors.ANCHOR_REQUIRED.hint"),
      ),
    );
    return false;
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

  const entryRow = (
    kind: "page" | "component",
    id: string,
    picked: boolean,
    candidateCount: number,
  ) => {
    const selected = state.view === (kind === "page" ? `page:${id}` : `component:${id}`);
    return (
      <div key={id} className="group/entry relative flex items-center">
        <button
          type="button"
          data-testid={`menu-${kind}-${id}`}
          aria-current={selected ? "true" : undefined}
          data-state={selected ? "current" : "default"}
          onClick={() => setView(kind === "page" ? `page:${id}` : `component:${id}`)}
          className={cn(
            "flex min-w-0 flex-1 items-center gap-1.5 rounded-md border border-transparent px-2 py-1 text-left font-mono text-xs transition-colors duration-150 ease-out",
            selected
              ? "border-primary/60 bg-primary/10 font-medium text-foreground"
              : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
          )}
        >
          <span
            aria-hidden="true"
            title={picked ? t("workspace.picked") : undefined}
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
          data-testid={`menu-${kind}-delete-${id}`}
          aria-label={t("common.delete")}
          title={t("common.delete")}
          onClick={() => (kind === "page" ? removePage(id) : removeComponent(id))}
          className="ml-0.5 flex size-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground opacity-0 transition-opacity duration-150 ease-out hover:bg-muted hover:text-destructive focus-visible:opacity-100 group-hover/entry:opacity-100"
        >
          <Trash2 className="size-3.5" />
        </button>
      </div>
    );
  };

  const groupHeader = (
    group: "pages" | "components",
    label: string,
    count: number,
    locked: boolean,
  ) => (
    <div className="flex h-7 items-center gap-1 px-1">
      <span
        className={cn(
          "min-w-0 flex-1 truncate text-[11px] font-semibold tracking-wide uppercase",
          locked ? "text-muted-foreground/60" : "text-muted-foreground",
        )}
      >
        {label}
      </span>
      <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{count}</span>
      <button
        type="button"
        data-testid={`menu-add-${group}`}
        aria-label={t(group === "pages" ? "workspace.addPage" : "workspace.addComponent")}
        title={locked ? t("workspace.lockedHint") : undefined}
        onClick={() => {
          if (group === "pages") {
            if (guardAdd("pages")) onAddPage();
          } else if (guardAdd("components")) {
            onAddComponent();
          }
        }}
        className={cn(
          "flex size-5 shrink-0 items-center justify-center rounded-sm transition-colors duration-150 ease-out",
          locked
            ? "cursor-default text-muted-foreground/50"
            : "text-muted-foreground hover:bg-muted hover:text-foreground",
        )}
      >
        <Plus className="size-3.5" />
      </button>
    </div>
  );

  return (
    <nav
      data-testid="left-menu"
      aria-label={t("workspace.menu")}
      className="flex w-60 shrink-0 flex-col gap-1 border-r p-2"
    >
      {confirmElement}
      <div className="relative">
        <Search
          aria-hidden="true"
          className="pointer-events-none absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
        />
        <Input
          data-testid="menu-search"
          value={keyword}
          onChange={(event) => setKeyword(event.target.value)}
          placeholder={t("workspace.searchPlaceholder")}
          className="h-8 pl-7 text-xs"
        />
      </div>

      <button
        type="button"
        data-testid="menu-overview"
        aria-current={state.view === "overview" ? "true" : undefined}
        data-state={state.view === "overview" ? "current" : "default"}
        onClick={() => setView("overview")}
        className={cn(
          "flex items-center gap-1.5 rounded-md border border-transparent px-2 py-1.5 text-left text-xs transition-colors duration-150 ease-out",
          state.view === "overview"
            ? "border-primary/60 bg-primary/10 font-medium text-foreground"
            : "text-foreground hover:bg-muted/60",
        )}
      >
        <Compass className="size-3.5 shrink-0 text-muted-foreground" />
        <span className="min-w-0 flex-1 truncate">{t("workspace.overview")}</span>
        {project.anchor && <AnchorBadge className="scale-90" />}
      </button>

      <div className="mt-1 border-t pt-1">
        {groupHeader("pages", t("workspace.pages"), project.pages.length, !unlocked.pages)}
        {unlocked.pages &&
          (pages.length > 0 ? (
            pages.map((page) =>
              entryRow("page", page.slug, page.current !== null, page.candidates.length),
            )
          ) : (
            <p className="px-2 py-1 text-[11px] text-muted-foreground">{t("workspace.emptyPages")}</p>
          ))}
      </div>

      <div className="mt-1 border-t pt-1">
        {groupHeader(
          "components",
          t("workspace.components"),
          project.components.length,
          !unlocked.components,
        )}
        {unlocked.components &&
          (components.length > 0 ? (
            components.map((component) =>
              entryRow(
                "component",
                component.name,
                component.current !== null,
                component.candidates.length,
              ),
            )
          ) : (
            <p className="px-2 py-1 text-[11px] text-muted-foreground">
              {t("workspace.emptyComponents")}
            </p>
          ))}
      </div>

      <div className="mt-auto flex items-center gap-1.5 px-1 pt-2 text-[11px] text-muted-foreground">
        <Boxes aria-hidden="true" className="size-3" />
        <span className="truncate">{project.name}</span>
        <LayoutGrid aria-hidden="true" className="hidden" />
      </div>
    </nav>
  );
}
