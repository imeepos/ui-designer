import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { EmptyState } from "@/components/empty-state";
import { BoardSection } from "@/components/gallery/board-section";
import { PagesSection } from "@/components/gallery/pages-section";
import { ComponentsSection } from "@/components/gallery/components-section";
import { ProjectOverview } from "@/components/gallery/project-overview";
import { Button } from "@/components/ui/button";

/** Center column: gallery content driven by the four-step view. */
export function GalleryPanel({ onNewProject }: { onNewProject: () => void }) {
  const { t } = useTranslation();
  const { state } = useStudio();
  const project = state.project;

  let content;
  if (!project) {
    content = (
      <EmptyState
        title={t("panel.gallery.empty.title")}
        description={t("panel.gallery.empty.desc")}
        action={
          <Button size="sm" data-testid="gallery-empty-cta" onClick={onNewProject}>
            {t("panel.gallery.empty.cta")}
          </Button>
        }
      />
    );
  } else {
    switch (state.view) {
      case "project":
        content = <ProjectOverview />;
        break;
      case "board":
        content = <BoardSection />;
        break;
      case "page":
        content = <PagesSection />;
        break;
      case "component":
        content = <ComponentsSection />;
        break;
    }
  }

  return (
    <section
      data-testid="gallery-panel"
      className="flex min-w-0 flex-1 flex-col"
    >
      <div className="flex h-10 shrink-0 items-center px-4">
        <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {t("panel.gallery.title")}
        </h2>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
        {content}
      </div>
    </section>
  );
}
