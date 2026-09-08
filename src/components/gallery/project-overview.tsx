import { ArrowRight, Boxes, FileImage, LayoutGrid } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { ArtImage } from "@/components/art-image";
import { AnchorBadge } from "@/components/card-actions";
import { Button } from "@/components/ui/button";
import { formatSize } from "@/lib/size";

/** Project view once a project exists: progress overview + go-to actions. */
export function ProjectOverview() {
  const { t } = useTranslation();
  const { state, setView } = useStudio();
  const project = state.project;
  if (!project) return null;

  const pagesDone = project.pages.filter((page) => page.current !== null).length;
  const componentsDone = project.components.filter(
    (component) => component.current !== null,
  ).length;

  return (
    <div className="flex flex-col gap-4" data-testid="project-overview">
      <section className="flex items-center gap-3 rounded-lg border bg-card p-3">
        {project.anchor ? (
          <>
            <ArtImage
              src={project.anchor.url}
              filter={project.anchor.filter}
              alt={t("gallery.board.anchorTitle")}
              className="h-20 w-32 shrink-0 rounded-md border"
            />
            <AnchorBadge />
          </>
        ) : (
          <p className="text-xs text-muted-foreground">{t("gallery.overview.anchorMissing")}</p>
        )}
        <div className="ml-auto flex items-center gap-3 font-mono text-xs text-muted-foreground">
          <span>{formatSize(project.size)}</span>
        </div>
      </section>

      <div className="grid grid-cols-3 gap-3">
        <SummaryCard
          icon={<FileImage className="size-4" />}
          label={t("gallery.overview.board")}
          value={
            project.anchor
              ? t("gallery.overview.done")
              : t("gallery.overview.candidates", { count: project.boardCandidates.length })
          }
          actionLabel={t("gallery.overview.gotoBoard")}
          onAction={() => setView("board")}
        />
        <SummaryCard
          icon={<LayoutGrid className="size-4" />}
          label={t("gallery.overview.pages")}
          value={t("gallery.overview.progress", { done: pagesDone, total: project.pages.length })}
          actionLabel={t("gallery.overview.gotoPages")}
          onAction={() => setView("page")}
        />
        <SummaryCard
          icon={<Boxes className="size-4" />}
          label={t("gallery.overview.components")}
          value={t("gallery.overview.progress", {
            done: componentsDone,
            total: project.components.length,
          })}
          actionLabel={t("gallery.overview.gotoComponents")}
          onAction={() => setView("component")}
        />
      </div>
    </div>
  );
}

function SummaryCard({
  icon,
  label,
  value,
  actionLabel,
  onAction,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  actionLabel: string;
  onAction: () => void;
}) {
  return (
    <div className="flex flex-col gap-2 rounded-lg border bg-card p-3">
      <span className="flex items-center gap-1.5 text-xs font-medium text-foreground">
        {icon}
        {label}
      </span>
      <span className="font-mono text-sm text-muted-foreground">{value}</span>
      <Button variant="ghost" size="sm" className="justify-start px-2" onClick={onAction}>
        {actionLabel}
        <ArrowRight className="size-3.5" />
      </Button>
    </div>
  );
}
