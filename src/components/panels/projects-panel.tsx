import { useState } from "react";
import { Plus } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { AnchorBadge } from "@/components/card-actions";
import { CreateProjectDialog } from "@/components/create-project-dialog";
import { Button } from "@/components/ui/button";
import { formatSize } from "@/lib/size";
import { cn } from "@/lib/utils";

/** Left column: project list + create (240px, THEME §4). */
export function ProjectsPanel() {
  const { t } = useTranslation();
  const { state, selectProject } = useStudio();
  const [createOpen, setCreateOpen] = useState(false);

  return (
    <aside
      data-testid="projects-panel"
      className="flex w-60 shrink-0 flex-col border-r"
    >
      <div className="flex h-10 shrink-0 items-center justify-between px-4">
        <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {t("panel.projects.title")}
        </h2>
        <Button size="sm" data-testid="new-project" onClick={() => setCreateOpen(true)}>
          <Plus className="size-3.5" />
          {t("panel.projects.new")}
        </Button>
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
        {state.projects.length === 0 ? (
          <div className="flex flex-1 items-center justify-center p-4">
            <p className="text-center text-xs text-muted-foreground">
              {t("panel.projects.empty")}
            </p>
          </div>
        ) : (
          <ul className="flex flex-col gap-1 p-2" data-testid="project-list">
            {state.projects.map((item) => {
              const active = state.project?.id === item.id;
              return (
                <li key={item.id}>
                  <button
                    type="button"
                    data-testid={`project-item-${item.id}`}
                    aria-current={active ? "true" : undefined}
                    onClick={() => void selectProject(item.id)}
                    className={cn(
                      "flex w-full flex-col gap-0.5 rounded-md border px-2.5 py-2 text-left transition-colors duration-150 ease-out",
                      active
                        ? "border-primary/60 bg-muted"
                        : "border-transparent hover:border-border hover:bg-muted/60",
                    )}
                  >
                    <span className="flex items-center gap-1.5">
                      <span className="min-w-0 flex-1 truncate text-xs font-medium text-foreground">
                        {item.name}
                      </span>
                      {item.hasAnchor && <AnchorBadge className="scale-90" />}
                    </span>
                    <span className="flex items-center gap-2 font-mono text-[10px] text-muted-foreground">
                      <span>{formatSize(item.size)}</span>
                      <span aria-hidden="true">·</span>
                      <span>{t("panel.projects.pageCount", { count: item.pageCount })}</span>
                      <span>{t("panel.projects.componentCount", { count: item.componentCount })}</span>
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      <CreateProjectDialog open={createOpen} onClose={() => setCreateOpen(false)} />
    </aside>
  );
}
