import { useTranslation } from "react-i18next";

import { EmptyState } from "@/components/empty-state";
import { Button } from "@/components/ui/button";

/** Center column: gallery (board candidates / pages / components), fluid width */
export function GalleryPanel() {
  const { t } = useTranslation();

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
      <div className="flex min-h-0 flex-1 flex-col">
        <EmptyState
          title={t("panel.gallery.empty.title")}
          description={t("panel.gallery.empty.desc")}
          action={
            <Button size="sm" data-testid="gallery-empty-cta">
              {t("panel.gallery.empty.cta")}
            </Button>
          }
        />
      </div>
    </section>
  );
}
