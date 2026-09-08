import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useStudio } from "@/state/studio";
import { ArtImage } from "@/components/art-image";
import {
  AnchorBadge,
  AnchorIcon,
  CardAction,
  EnlargeIcon,
  PickIcon,
  TrashIcon,
} from "@/components/card-actions";
import { ArtifactCard } from "@/components/artifact-card";
import { EmptyState } from "@/components/empty-state";
import { SkeletonGrid } from "@/components/gallery/skeleton-grid";
import { useConfirm } from "@/hooks/use-confirm";
import { HelmMark } from "@/components/icons";
import { Lightbox } from "@/components/lightbox";

interface LightboxTarget {
  url: string;
  filter?: string;
  label: string;
}

/** Board view: anchor strip + candidate grid (step 2). */
export function BoardSection() {
  const { t } = useTranslation();
  const { state, pickAnchor, deleteArtifact } = useStudio();
  const project = state.project;
  const { ask, element: confirmElement } = useConfirm();
  const [lightbox, setLightbox] = useState<LightboxTarget | null>(null);

  if (!project) return null;

  const anchor = project.anchor;

  return (
    <div className="flex flex-col gap-4" data-testid="board-section">
      {confirmElement}
      <Lightbox
        open={lightbox !== null}
        onClose={() => setLightbox(null)}
        imageUrl={lightbox?.url ?? ""}
        filter={lightbox?.filter}
        alt={lightbox?.label ?? ""}
        caption={lightbox?.label}
      />

      <section
        data-testid="anchor-strip"
        className="flex items-center gap-3 rounded-lg border bg-card p-3"
      >
        {anchor ? (
          <>
            <ArtImage
              src={anchor.url}
              filter={anchor.filter}
              alt={t("gallery.board.anchorTitle")}
              className="h-20 w-32 shrink-0 rounded-md border"
            />
            <div className="flex min-w-0 flex-col gap-1">
              <AnchorBadge />
              <p className="truncate text-xs text-muted-foreground">
                {t("gallery.board.anchorDesc")}
              </p>
              <p className="font-mono text-[11px] text-muted-foreground">
                {anchor.candidateId}
              </p>
            </div>
          </>
        ) : (
          <>
            <HelmMark className="size-8 shrink-0 text-muted-foreground" />
            <p className="text-xs text-muted-foreground">{t("gallery.board.anchorEmpty")}</p>
          </>
        )}
      </section>

      <section className="flex flex-col gap-2">
        <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {t("gallery.section.board")}
        </h3>
        {project.boardCandidates.length === 0 && state.job?.kind !== "board" ? (
          <EmptyState
            title={t("gallery.board.empty.title")}
            description={t("gallery.board.empty.desc")}
            className="py-10"
          />
        ) : (
          <div className="grid grid-cols-2 gap-3 xl:grid-cols-3" data-testid="board-candidates">
            {project.boardCandidates.map((candidate) => {
              const isAnchor = anchor?.candidateId === candidate.id;
              return (
                <ArtifactCard
                  key={candidate.id}
                  testId={`board-candidate-${candidate.id}`}
                  imageUrl={candidate.url}
                  filter={candidate.filter}
                  alt={t("gallery.board.candidateAlt", { id: candidate.id })}
                  aspect="square"
                  selected={isAnchor}
                  badge={isAnchor ? <AnchorBadge /> : undefined}
                  footer={
                    <>
                      <span className="truncate font-mono">{candidate.id}</span>
                      <span className="ml-auto shrink-0 font-mono">
                        {formatTime(candidate.createdAt)}
                      </span>
                    </>
                  }
                  actions={
                    <>
                      <CardAction
                        label={t("common.enlarge")}
                        testId="action-enlarge"
                        onClick={() =>
                          setLightbox({
                            url: candidate.url,
                            filter: candidate.filter,
                            label: t("gallery.board.candidateAlt", { id: candidate.id }),
                          })
                        }
                      >
                        <EnlargeIcon />
                      </CardAction>
                      {!isAnchor && (
                        <CardAction
                          label={t("gallery.board.setAnchor")}
                          primary
                          testId="action-set-anchor"
                          onClick={() => void pickAnchor(candidate.id)}
                        >
                          <AnchorIcon />
                        </CardAction>
                      )}
                      <CardAction
                        label={t("common.delete")}
                        destructive
                        testId="action-delete"
                        onClick={() =>
                          ask({
                            title: t("confirm.deleteCandidate.title"),
                            description: t("confirm.deleteCandidate.desc", {
                              id: candidate.id,
                            }),
                            onConfirm: () =>
                              void deleteArtifact({
                                kind: "boardCandidate",
                                candidateId: candidate.id,
                              }),
                          })
                        }
                      >
                        <TrashIcon />
                      </CardAction>
                    </>
                  }
                />
              );
            })}
            {state.job?.kind === "board" && <SkeletonGrid count={2} />}
          </div>
        )}
      </section>
      <p className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
        <PickIcon className="size-3" />
        {t("gallery.board.pickHint")}
      </p>
    </div>
  );
}

export function formatTime(ts: number): string {
  const date = new Date(ts);
  const pad = (value: number) => value.toString().padStart(2, "0");
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}
