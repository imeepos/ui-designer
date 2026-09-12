import { useEffect, useState } from "react";
import { Check, ZoomIn } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { Candidate } from "@/lib/api/types";
import { useStudio } from "@/state/studio";
import { ArtImage } from "@/components/art-image";
import { Lightbox } from "@/components/lightbox";
import { Button } from "@/components/ui/button";
import { Modal } from "@/components/ui/modal";
import { cn } from "@/lib/utils";

export type SwitchMode =
  | "board"
  | { kind: "page"; slug: string }
  | { kind: "component"; name: string };

/**
 * Switch-draft dialog: candidate thumbnails with preview + single select;
 * confirming performs the pick so the stage image swaps immediately and the
 * old current moves to history (PRD §3.3/§3.4 pick semantics).
 */
export function SwitchDialog({ mode, onClose }: { mode: SwitchMode; onClose: () => void }) {
  const { t } = useTranslation();
  const { state, pickAnchor, pickPage, pickComponent } = useStudio();
  const project = state.project;
  const [selected, setSelected] = useState<string | null>(null);
  const [preview, setPreview] = useState<Candidate | null>(null);

  const candidates = !project
    ? []
    : mode === "board"
      ? project.boardCandidates
      : mode.kind === "page"
        ? (project.pages.find((item) => item.slug === mode.slug)?.candidates ?? [])
        : (project.components.find((item) => item.name === mode.name)?.candidates ?? []);

  useEffect(() => {
    setSelected(null);
    setPreview(null);
  }, [mode]);

  if (!project) return null;

  const confirm = async () => {
    if (!selected) return;
    if (mode === "board") {
      await pickAnchor(selected);
    } else if (mode.kind === "page") {
      await pickPage(mode.slug, selected);
    } else {
      await pickComponent(mode.name, selected);
    }
    onClose();
  };

  const title =
    mode === "board"
      ? t("workspace.switchAnchor")
      : mode.kind === "page"
        ? t("switch.titlePage", { name: mode.slug })
        : t("switch.titleComponent", { name: mode.name });

  return (
    <Modal
      open
      onClose={onClose}
      title={title}
      testId="switch-dialog"
      wide
      footer={
        <>
          <span className="mr-auto text-[11px] text-muted-foreground">
            {t("switch.historyHint")}
          </span>
          <Button variant="outline" size="sm" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button size="sm" data-testid="switch-confirm" disabled={!selected} onClick={() => void confirm()}>
            {t("switch.confirm")}
          </Button>
        </>
      }
    >
      {candidates.length === 0 ? (
        <p className="py-10 text-center text-sm text-muted-foreground">{t("switch.empty")}</p>
      ) : (
        <div className="grid grid-cols-3 gap-3" data-testid="switch-grid">
          {candidates.map((candidate) => {
            const isSelected = selected === candidate.id;
            return (
              <div
                key={candidate.id}
                data-testid={`switch-candidate-${candidate.id}`}
                className={cn(
                  "group relative overflow-hidden rounded-md border transition-colors duration-150 ease-out",
                  isSelected
                    ? "border-primary ring-1 ring-primary"
                    : "border-border hover:border-primary/60",
                )}
              >
                <button
                  type="button"
                  aria-pressed={isSelected}
                  onClick={() => setSelected(candidate.id)}
                  className="block w-full"
                >
                  <ArtImage
                    src={candidate.url}
                    filter={candidate.filter}
                    alt={t("gallery.board.candidateAlt", { id: candidate.id })}
                    className="aspect-video w-full"
                  />
                </button>
                {isSelected && (
                  <span className="absolute left-1.5 top-1.5 flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground">
                    <Check className="size-3" />
                  </span>
                )}
                <button
                  type="button"
                  data-testid={`switch-preview-${candidate.id}`}
                  aria-label={t("common.enlarge")}
                  title={t("common.enlarge")}
                  onClick={() => setPreview(candidate)}
                  className="absolute right-1.5 top-1.5 flex size-6 items-center justify-center rounded-md border bg-background/90 text-muted-foreground opacity-0 transition-opacity duration-150 ease-out hover:text-foreground group-hover:opacity-100"
                >
                  <ZoomIn className="size-3.5" />
                </button>
                <span className="block truncate border-t px-2 py-1 font-mono text-[11px] text-muted-foreground">
                  {candidate.id}
                </span>
              </div>
            );
          })}
        </div>
      )}

      {preview && (
        <Lightbox
          open
          onClose={() => setPreview(null)}
          imageUrl={preview.url}
          filter={preview.filter}
          alt={t("gallery.board.candidateAlt", { id: preview.id })}
          caption={preview.id}
        />
      )}
    </Modal>
  );
}
