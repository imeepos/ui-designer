import { ImagePlus, RefreshCw, Repeat, Scan, Trash2, Waypoints } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { InfiniteCanvas } from "@/components/infinite-canvas";
import { EmptyState } from "@/components/empty-state";
import { Button } from "@/components/ui/button";
import { useConfirm } from "@/hooks/use-confirm";
import type { LineageTarget } from "@/lib/api/types";

/** Zoom bounds for the embedded stage canvas: 1%–6400%. */
const MIN_SCALE = 0.01;
const MAX_SCALE = 64;

/**
 * Center stage: the picked artwork of the selected menu entry (anchor for
 * overview, current for pages/components) on an infinite canvas — wheel
 * zooms at the cursor (1%–6400%), drag pans, double-click or the corner
 * "fit" button resets the view. Blood-lineage and top-right actions float
 * above the canvas as overlays. Geometry lives in the shared
 * InfiniteCanvas (same component as the fullscreen lightbox).
 */
export function Stage({
  onRegenerate,
  onSwitch,
  onLineage,
}: {
  onRegenerate: () => void;
  onSwitch: () => void;
  onLineage: (target: LineageTarget) => void;
}) {
  const { t } = useTranslation();
  const { state, deleteArtifact } = useStudio();
  const { ask, element: confirmElement } = useConfirm();
  const project = state.project;
  if (!project) return null;

  const view = state.view;
  const isOverview = view === "overview";
  const page = view.startsWith("page:")
    ? project.pages.find((item) => item.slug === view.slice("page:".length))
    : undefined;
  const component = view.startsWith("component:")
    ? project.components.find((item) => item.name === view.slice("component:".length))
    : undefined;

  const current = isOverview
    ? (project.anchor ?? null)
    : page
      ? page.current
      : component
        ? component.current
        : null;
  const candidateCount = isOverview
    ? project.boardCandidates.length
    : (page?.candidates.length ?? component?.candidates.length ?? 0);
  const title = isOverview
    ? t("workspace.overview")
    : (page?.slug ?? component?.name ?? "");
  const lineageTarget: LineageTarget | null = !current
    ? null
    : isOverview
      ? { kind: "board", candidateId: current.candidateId }
      : page
        ? { kind: "page", slug: page.slug, candidateId: current.candidateId }
        : component
          ? { kind: "component", name: component.name, candidateId: current.candidateId }
          : null;

  const removeEntry = () => {
    if (page) {
      ask({
        title: t("confirm.deleteItem.title"),
        description: t("confirm.deleteItem.desc", { name: page.slug }),
        onConfirm: () => void deleteArtifact({ kind: "page", slug: page.slug }),
      });
    } else if (component) {
      ask({
        title: t("confirm.deleteItem.title"),
        description: t("confirm.deleteItem.desc", { name: component.name }),
        onConfirm: () => void deleteArtifact({ kind: "component", name: component.name }),
      });
    }
  };

  const actionButtons = (
    <>
      <Button
        variant="outline"
        size="sm"
        data-testid="stage-regenerate"
        onClick={onRegenerate}
      >
        <RefreshCw className="size-3.5" />
        {t("workspace.regenerate")}
      </Button>
      <Button
        variant="outline"
        size="sm"
        data-testid="stage-switch"
        disabled={candidateCount === 0}
        onClick={onSwitch}
      >
        <Repeat className="size-3.5" />
        {isOverview ? t("workspace.switchAnchor") : t("workspace.switchDraft")}
      </Button>
      <Button
        variant="ghost"
        size="icon"
        data-testid="stage-lineage"
        aria-label={t("lineage.open")}
        title={t("lineage.open")}
        disabled={!lineageTarget}
        onClick={() => {
          if (lineageTarget) onLineage(lineageTarget);
        }}
      >
        <Waypoints className="size-4" />
      </Button>
      {!isOverview && (
        <Button
          variant="ghost"
          size="icon"
          data-testid="stage-delete"
          aria-label={t("common.delete")}
          title={t("common.delete")}
          className="text-destructive hover:text-destructive"
          onClick={removeEntry}
        >
          <Trash2 className="size-4" />
        </Button>
      )}
    </>
  );

  return (
    <section className="relative flex min-w-0 flex-1 flex-col" data-testid="stage">
      {confirmElement}

      {current ? (
        <InfiniteCanvas
          imageUrl={current.url}
          filter={current.filter}
          alt={title}
          minScale={MIN_SCALE}
          maxScale={MAX_SCALE}
          doubleClick="fit"
          className="absolute inset-0"
          canvasTestId="stage-canvas"
          imageTestId="stage-image"
          loadFailedText={t("lightbox.loadFailed")}
        >
          {({ percent, actions }) => (
            <>
              {/* Zoom badge + reset, bottom-right */}
              <div className="absolute bottom-3 right-3 flex items-center gap-0.5 rounded-lg border bg-card p-1 shadow-[0_8px_24px_rgba(2,8,23,0.08)]">
                <span
                  data-testid="stage-zoom-level"
                  className="min-w-14 px-1 text-center font-mono text-xs text-muted-foreground"
                >
                  {percent}
                </span>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={t("stage.fit")}
                  title={t("stage.fit")}
                  data-testid="stage-fit"
                  onClick={actions.fit}
                >
                  <Scan className="size-4" />
                </Button>
              </div>
            </>
          )}
        </InfiniteCanvas>
      ) : (
        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
          <EmptyState
            title={
              isOverview
                ? t("workspace.stageEmpty.overview")
                : candidateCount > 0
                  ? t("workspace.stageEmpty.unpicked")
                  : t("workspace.stageEmpty.entry")
            }
            description={
              isOverview
                ? t("workspace.stageEmpty.overviewDesc")
                : t("workspace.stageEmpty.entryDesc")
            }
            action={
              candidateCount > 0 && !isOverview ? (
                <Button size="sm" data-testid="stage-empty-switch" onClick={onSwitch}>
                  {t("workspace.switchDraft")}
                </Button>
              ) : (
                <Button size="sm" data-testid="stage-empty-regenerate" onClick={onRegenerate}>
                  <ImagePlus className="size-3.5" />
                  {t("workspace.regenerate")}
                </Button>
              )
            }
          />
        </div>
      )}

      {/* Actions, top-right — floating above the canvas AND available in the
          empty state (regenerate/switch must stay reachable before a pick). */}
      <div className="absolute right-3 top-3 z-10 flex items-center gap-1 rounded-lg border bg-card p-1 shadow-[0_8px_24px_rgba(2,8,23,0.08)]">
        {actionButtons}
      </div>
    </section>
  );
}
