import { useState, type ReactNode } from "react";
import { ImagePlus } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { Candidate, HistoryEntry } from "@/lib/api/types";
import {
  CardAction,
  EnlargeIcon,
  PickIcon,
  TrashIcon,
} from "@/components/card-actions";
import { ArtifactCard } from "@/components/artifact-card";
import { EmptyState } from "@/components/empty-state";
import { SkeletonGrid } from "@/components/gallery/skeleton-grid";
import { formatTime } from "@/components/gallery/board-section";
import { useConfirm } from "@/hooks/use-confirm";
import { Lightbox } from "@/components/lightbox";
import { cn } from "@/lib/utils";

export interface FlowItem {
  id: string;
  label: string;
  current: { url: string; candidateId: string; filter?: string } | null;
  candidates: Candidate[];
  history: HistoryEntry[];
  meta?: ReactNode;
}

interface LightboxTarget {
  url: string;
  filter?: string;
  label: string;
}

interface ArtifactFlowProps {
  testPrefix: string;
  items: FlowItem[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onPick: (id: string, candidateId: string) => void;
  onDeleteItem: (id: string) => void;
  onDeleteHistory: (id: string, ts: number) => void;
  jobTarget: string | undefined;
  emptyTitle: string;
  emptyDesc: string;
}

/**
 * Shared page/component lifecycle: item grid -> candidate comparison ->
 * pick -> history strip (PRD §3.3/§3.4).
 */
export function ArtifactFlow({
  testPrefix,
  items,
  selectedId,
  onSelect,
  onPick,
  onDeleteItem,
  onDeleteHistory,
  jobTarget,
  emptyTitle,
  emptyDesc,
}: ArtifactFlowProps) {
  const { t } = useTranslation();
  const { ask, element: confirmElement } = useConfirm();
  const [lightbox, setLightbox] = useState<LightboxTarget | null>(null);
  const selected = items.find((item) => item.id === selectedId) ?? null;

  return (
    <div className="flex flex-col gap-4" data-testid={`${testPrefix}-section`}>
      {confirmElement}
      <Lightbox
        open={lightbox !== null}
        onClose={() => setLightbox(null)}
        imageUrl={lightbox?.url ?? ""}
        filter={lightbox?.filter}
        alt={lightbox?.label ?? ""}
        caption={lightbox?.label}
      />

      <section className="flex flex-col gap-2">
        <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
          {t("gallery.flow.items")}
        </h3>
        {items.length === 0 ? (
          <EmptyState title={emptyTitle} description={emptyDesc} className="py-10" />
        ) : (
          <div className="grid grid-cols-2 gap-3 xl:grid-cols-3" data-testid={`${testPrefix}-items`}>
            {items.map((item) => (
              <ArtifactCard
                key={item.id}
                testId={`${testPrefix}-item-${item.id}`}
                imageUrl={item.current?.url ?? ""}
                filter={item.current?.filter}
                alt={item.label}
                aspect="video"
                selected={item.id === selectedId}
                badge={
                  item.current ? (
                    <span
                      data-testid={`${testPrefix}-picked-badge`}
                      className="inline-flex items-center gap-1 rounded-full bg-primary px-2 py-0.5 text-[10px] font-semibold text-primary-foreground"
                    >
                      <PickIcon />
                      {t("gallery.flow.picked")}
                    </span>
                  ) : undefined
                }
                onClick={() => onSelect(item.id)}
                placeholder={!item.current ? <EmptyThumb /> : undefined}
                footer={
                  <>
                    <span className="min-w-0 truncate font-medium text-foreground">{item.label}</span>
                    {item.meta}
                    <span className="ml-auto shrink-0">
                      {statusLabel(t, item.current != null, item.candidates.length)}
                    </span>
                  </>
                }
                actions={
                  <>
                    {item.current && (
                      <CardAction
                        label={t("common.enlarge")}
                        testId="action-enlarge"
                        onClick={() =>
                          setLightbox({
                            url: item.current!.url,
                            filter: item.current!.filter,
                            label: item.label,
                          })
                        }
                      >
                        <EnlargeIcon />
                      </CardAction>
                    )}
                    <CardAction
                      label={t("common.delete")}
                      destructive
                      testId="action-delete"
                      onClick={() =>
                        ask({
                          title: t("confirm.deleteItem.title"),
                          description: t("confirm.deleteItem.desc", { name: item.label }),
                          onConfirm: () => onDeleteItem(item.id),
                        })
                      }
                    >
                      <TrashIcon />
                    </CardAction>
                  </>
                }
              />
            ))}
          </div>
        )}
      </section>

      {selected && (
        <section className="flex flex-col gap-2" data-testid={`${testPrefix}-candidates-zone`}>
          <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
            {t("gallery.flow.candidates", { name: selected.label })}
          </h3>
          {selected.candidates.length === 0 && jobTarget !== selected.id ? (
            <p className="rounded-md border border-dashed px-3 py-4 text-center text-xs text-muted-foreground">
              {t("gallery.flow.candidatesEmpty")}
            </p>
          ) : (
            <div
              className="grid grid-cols-2 gap-3 xl:grid-cols-3"
              data-testid={`${testPrefix}-candidates`}
            >
              {selected.candidates.map((candidate) => (
                <ArtifactCard
                  key={candidate.id}
                  testId={`${testPrefix}-candidate-${candidate.id}`}
                  imageUrl={candidate.url}
                  filter={candidate.filter}
                  alt={t("gallery.flow.candidateAlt", { id: candidate.id })}
                  aspect="video"
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
                            label: `${selected.label} · ${candidate.id}`,
                          })
                        }
                      >
                        <EnlargeIcon />
                      </CardAction>
                      <CardAction
                        label={t("gallery.flow.pick")}
                        primary
                        testId="action-pick"
                        onClick={() => onPick(selected.id, candidate.id)}
                      >
                        <PickIcon />
                      </CardAction>
                    </>
                  }
                />
              ))}
              {jobTarget === selected.id && <SkeletonGrid count={2} aspect="video" />}
            </div>
          )}
        </section>
      )}

      {selected && selected.history.length > 0 && (
        <section className="flex flex-col gap-2" data-testid={`${testPrefix}-history`}>
          <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
            {t("gallery.flow.history", { count: selected.history.length })}
          </h3>
          <div className="flex gap-2 overflow-x-auto pb-1">
            {[...selected.history].reverse().map((entry) => (
              <div
                key={entry.ts}
                data-testid={`${testPrefix}-history-item`}
                className="group relative shrink-0"
              >
                <LightThumb
                  url={entry.url}
                  filter={entry.filter}
                  label={`${selected.label} · ${formatTime(entry.ts)}`}
                  onEnlarge={() =>
                    setLightbox({
                      url: entry.url,
                      filter: entry.filter,
                      label: `${selected.label} · ${formatTime(entry.ts)}`,
                    })
                  }
                />
                <div className="absolute top-1 right-1 opacity-0 transition-opacity duration-150 ease-out group-hover:opacity-100">
                  <CardAction
                    label={t("common.delete")}
                    destructive
                    testId="action-delete"
                    onClick={() =>
                      ask({
                        title: t("confirm.deleteHistory.title"),
                        description: t("confirm.deleteHistory.desc", {
                          name: selected.label,
                          time: formatTime(entry.ts),
                        }),
                        onConfirm: () => onDeleteHistory(selected.id, entry.ts),
                      })
                    }
                  >
                    <TrashIcon />
                  </CardAction>
                </div>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function statusLabel(
  t: (key: string, options?: Record<string, unknown>) => string,
  picked: boolean,
  candidateCount: number,
): string {
  if (picked) return t("gallery.flow.pickedShort");
  if (candidateCount > 0) return t("gallery.flow.candidatesShort", { count: candidateCount });
  return t("gallery.flow.untouched");
}

function EmptyThumb() {
  const { t } = useTranslation();
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center gap-1.5 border border-dashed bg-muted/40 text-muted-foreground">
      <ImagePlus aria-hidden="true" className="size-5" />
      <span className="text-[10px]">{t("gallery.flow.placeholder")}</span>
    </div>
  );
}

function LightThumb({
  url,
  filter,
  label,
  onEnlarge,
  className,
}: {
  url: string;
  filter?: string;
  label: string;
  onEnlarge: () => void;
  className?: string;
}) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      title={t("common.enlarge")}
      aria-label={t("common.enlarge")}
      onClick={onEnlarge}
      className={cn(
        "block w-32 overflow-hidden rounded-md border bg-card transition-colors duration-150 ease-out hover:border-primary",
        className,
      )}
    >
      <img
        src={url}
        alt={label}
        draggable={false}
        loading="lazy"
        style={filter && filter !== "none" ? { filter } : undefined}
        className="aspect-video w-full object-cover"
      />
    </button>
  );
}
