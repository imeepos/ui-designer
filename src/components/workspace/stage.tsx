import { useState } from "react";
import { ImagePlus, RefreshCw, Repeat, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { ArtImage } from "@/components/art-image";
import { AnchorBadge } from "@/components/card-actions";
import { EmptyState } from "@/components/empty-state";
import { Button } from "@/components/ui/button";
import { useConfirm } from "@/hooks/use-confirm";
import { Lightbox } from "@/components/lightbox";

/**
 * Center stage: the picked artwork of the selected menu entry (anchor for
 * overview, current for pages/components) with its two top-right actions.
 */
export function Stage({
  onRegenerate,
  onSwitch,
}: {
  onRegenerate: () => void;
  onSwitch: () => void;
}) {
  const { t } = useTranslation();
  const { state, deleteArtifact } = useStudio();
  const { ask, element: confirmElement } = useConfirm();
  const [lightboxUrl, setLightboxUrl] = useState<string | null>(null);
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
  const brief = isOverview ? project.brandBrief : (page?.brief ?? component?.brief ?? "");

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

  return (
    <section className="flex min-w-0 flex-1 flex-col" data-testid="stage">
      {confirmElement}
      <Lightbox
        open={lightboxUrl !== null}
        onClose={() => setLightboxUrl(null)}
        imageUrl={lightboxUrl ?? ""}
        alt={title}
        caption={title}
      />

      <div className="flex h-11 shrink-0 items-center gap-2 border-b px-4">
        <h2 className="min-w-0 flex-1 truncate font-mono text-sm font-semibold text-foreground">
          {title}
        </h2>
        {current && (
          <Button
            variant="outline"
            size="sm"
            data-testid="stage-regenerate"
            onClick={onRegenerate}
          >
            <RefreshCw className="size-3.5" />
            {t("workspace.regenerate")}
          </Button>
        )}
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
      </div>

      <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-4 overflow-y-auto p-6">
        {current ? (
          <>
            <button
              type="button"
              data-testid="stage-image"
              title={t("common.enlarge")}
              onClick={() => setLightboxUrl(current.url)}
              className="max-h-full max-w-full overflow-hidden rounded-lg border bg-card transition-colors duration-150 ease-out hover:border-primary"
            >
              <ArtImage
                src={current.url}
                filter={current.filter}
                alt={title}
                className="max-h-[70vh] w-auto max-w-full object-contain"
              />
            </button>
            <div className="flex w-full max-w-xl flex-col items-center gap-1 text-center">
              {isOverview && <AnchorBadge />}
              {brief && (
                <p className="line-clamp-2 text-xs text-muted-foreground">{brief}</p>
              )}
              <p className="font-mono text-[11px] text-muted-foreground">{current.candidateId}</p>
            </div>
          </>
        ) : (
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
        )}
      </div>
    </section>
  );
}
