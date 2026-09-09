import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { AnchorBadge } from "@/components/card-actions";
import { CreateProjectDialog } from "@/components/create-project-dialog";
import { ProjectTree } from "@/components/panels/project-tree";
import { cn } from "@/lib/utils";

/**
 * Left column (240px, THEME §4): compact project switcher on top, directory
 * tree of the active project below (replaces the horizontal stepper).
 */
export function ProjectsPanel() {
  const { t } = useTranslation();
  const { state, selectProject } = useStudio();
  const [createOpen, setCreateOpen] = useState(false);
  const openCreate = () => setCreateOpen(true);

  return (
    <aside
      data-testid="projects-panel"
      className="flex w-60 shrink-0 flex-col border-r"
    >
      <div className="flex h-10 shrink-0 items-center justify-between px-4">
        <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {t("panel.projects.title")}
        </h2>
        <button
          type="button"
          data-testid="new-project"
          onClick={openCreate}
          className="rounded-sm px-1.5 py-0.5 text-xs font-medium text-primary transition-colors duration-150 ease-out hover:bg-muted"
        >
          {t("panel.projects.new")}
        </button>
      </div>

      {state.projects.length === 0 ? (
        <div className="flex shrink-0 items-center px-4 pb-2">
          <p className="text-[11px] text-muted-foreground">{t("panel.projects.empty")}</p>
        </div>
      ) : (
        <ul className="flex max-h-40 shrink-0 flex-col gap-0.5 overflow-y-auto p-2 pt-0" data-testid="project-list">
          {state.projects.map((item) => {
            const active = state.project?.id === item.id;
            return (
              <li key={item.id}>
                <button
                  type="button"
                  data-testid={`project-item-${item.id}`}
                  aria-current={active ? "true" : undefined}
                  title={`${item.name} · ${t("panel.projects.pageCount", { count: item.pageCount })} ${t("panel.projects.componentCount", { count: item.componentCount })}`}
                  onClick={() => void selectProject(item.id)}
                  className={cn(
                    "flex w-full items-center gap-1.5 rounded-md border px-2 py-1 text-left transition-colors duration-150 ease-out",
                    active
                      ? "border-primary/60 bg-muted"
                      : "border-transparent hover:border-border hover:bg-muted/60",
                  )}
                >
                  <span className="min-w-0 flex-1 truncate text-xs text-foreground">
                    {item.name}
                  </span>
                  {item.hasAnchor && <AnchorBadge className="scale-90" />}
                </button>
              </li>
            );
          })}
        </ul>
      )}

      <div className="flex h-8 shrink-0 items-center border-t px-4">
        <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {t("tree.title")}
        </h2>
      </div>

      <ProjectTree onNewProject={openCreate} />

      <CreateProjectDialog open={createOpen} onClose={() => setCreateOpen(false)} />
    </aside>
  );
}
