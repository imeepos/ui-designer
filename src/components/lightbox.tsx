import { useEffect } from "react";
import { Minus, Plus, Scan, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { InfiniteCanvas } from "@/components/infinite-canvas";
import { Button } from "@/components/ui/button";

export type LightboxProps = {
  open: boolean;
  onClose: () => void;
  imageUrl: string;
  filter?: string;
  alt: string;
  caption?: string;
};

const MIN_SCALE = 0.04;
const MAX_SCALE = 32;

/**
 * Fullscreen infinite-canvas preview (THEME §5 "enlarge" action).
 *
 * A thin owner shell over the shared [`InfiniteCanvas`]: fullscreen dialog
 * frame, header card + close button, and the floating footer toolbar
 * (−/percent/+/fit/1:1) wired to the canvas actions. The zoom bounds are
 * 4%–3200% with nearest-neighbor ≥3× for pixel-level checking; geometry
 * (cursor-anchored wheel zoom, drag pan, double-click fit ↔ 1:1, dot grid)
 * lives in the shared component, same as the workspace stage.
 */
export function Lightbox({ open, onClose, imageUrl, filter, alt, caption }: LightboxProps) {
  const { t } = useTranslation();
  // Lock body scroll for the duration of the fullscreen overlay.
  useEffect(() => {
    if (!open) return;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "";
    };
  }, [open]);

  if (!open) return null;

  return (
    <div
      data-testid="lightbox"
      role="dialog"
      aria-modal="true"
      aria-label={caption ?? alt}
      className="fixed inset-0 z-50 bg-background"
    >
      <InfiniteCanvas
        imageUrl={imageUrl}
        filter={filter}
        alt={alt}
        minScale={MIN_SCALE}
        maxScale={MAX_SCALE}
        doubleClick="toggleFitActual"
        pixelatedAt={3}
        checkerboard
        onEscape={onClose}
        className="absolute inset-0"
        canvasTestId="lightbox-canvas"
        imageTestId="lightbox-image"
        loadFailedText={t("lightbox.loadFailed")}
      >
        {({ percent, loaded, actions }) => (
          <>
            <header className="pointer-events-none absolute inset-x-0 top-0 flex items-start justify-between gap-4 p-4">
              <div className="pointer-events-auto max-w-[min(560px,70vw)] rounded-lg border bg-card px-3 py-2 shadow-[0_8px_24px_rgba(2,8,23,0.08)]">
                <p className="truncate text-sm font-semibold">{caption ?? alt}</p>
                <p className="font-mono text-[11px] text-muted-foreground">{t("lightbox.hint")}</p>
              </div>
              <Button
                variant="ghost"
                size="icon"
                aria-label={t("common.close")}
                data-testid="lightbox-close"
                onClick={onClose}
                className="pointer-events-auto border bg-card shadow-[0_8px_24px_rgba(2,8,23,0.08)]"
              >
                <X className="size-4" />
              </Button>
            </header>

            <footer className="pointer-events-none absolute inset-x-0 bottom-5 flex justify-center">
              <div className="pointer-events-auto flex items-center gap-0.5 rounded-xl border bg-card p-1 shadow-[0_8px_24px_rgba(2,8,23,0.08)]">
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={t("lightbox.zoomOut")}
                  data-testid="lightbox-zoom-out"
                  disabled={!loaded}
                  onClick={actions.zoomOut}
                >
                  <Minus className="size-4" />
                </Button>
                <button
                  type="button"
                  data-testid="lightbox-zoom-level"
                  title={t("lightbox.actualSize")}
                  disabled={!loaded}
                  onClick={actions.actualSize}
                  className="min-w-14 rounded-sm px-1 py-1 text-center font-mono text-xs text-muted-foreground outline-none transition-colors duration-150 ease-out hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50"
                >
                  {loaded ? percent : "–"}
                </button>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={t("lightbox.zoomIn")}
                  data-testid="lightbox-zoom-in"
                  disabled={!loaded}
                  onClick={actions.zoomIn}
                >
                  <Plus className="size-4" />
                </Button>
                <div className="mx-1 h-5 w-px bg-border" />
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={t("lightbox.fit")}
                  title={t("lightbox.fit")}
                  data-testid="lightbox-fit"
                  disabled={!loaded}
                  onClick={actions.fit}
                >
                  <Scan className="size-4" />
                </Button>
                <Button
                  variant="ghost"
                  aria-label={t("lightbox.actualSize")}
                  title={t("lightbox.actualSize")}
                  data-testid="lightbox-actual"
                  disabled={!loaded}
                  onClick={actions.actualSize}
                  className="px-2 font-mono text-xs"
                >
                  1:1
                </Button>
              </div>
            </footer>
          </>
        )}
      </InfiniteCanvas>
    </div>
  );
}
