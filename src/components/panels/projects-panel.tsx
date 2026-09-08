import { Plus } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";

/** Left column: project list + create (240px, THEME §4) */
export function ProjectsPanel() {
  const { t } = useTranslation();

  return (
    <aside
      data-testid="projects-panel"
      className="flex w-60 shrink-0 flex-col border-r"
    >
      <div className="flex h-10 shrink-0 items-center justify-between px-4">
        <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {t("panel.projects.title")}
        </h2>
        <Button size="sm" data-testid="new-project">
          <Plus className="size-3.5" />
          {t("panel.projects.new")}
        </Button>
      </div>
      <div className="flex flex-1 items-center justify-center p-4">
        <p className="text-center text-xs text-muted-foreground">
          {t("panel.projects.empty")}
        </p>
      </div>
    </aside>
  );
}
