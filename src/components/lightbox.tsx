import { useCallback, useEffect, useRef, useState } from "react";
import { Minus, Plus, Scan, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

export type LightboxProps = {
  open: boolean;
  onClose: () => void;
  imageUrl: string;
  filter?: string;
  alt: string;
  caption?: string;
};

type View = { scale: number; x: number; y: number };

const MIN_SCALE = 0.04;
const MAX_SCALE = 32;
const ZOOM_STEP = 1.2;
const WHEEL_FACTOR = 0.0015;
/** Zoom ≥ this renders nearest-neighbor for true pixel inspection. */
const PIXELATED_AT = 3;
const DOT_GRID = 24;

const clampScale = (scale: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));

const fitView = (natural: { w: number; h: number }, viewport: { w: number; h: number }): View => {
  const scale = clampScale(Math.min((viewport.w - 64) / natural.w, (viewport.h - 64) / natural.h));
  return {
    scale,
    x: (viewport.w - natural.w * scale) / 2,
    y: (viewport.h - natural.h * scale) / 2,
  };
};

const formatPercent = (scale: number): string => {
  const percent = scale * 100;
  return `${percent >= 10 ? Math.round(percent) : percent.toFixed(1)}%`;
};

/**
 * Fullscreen infinite-canvas preview (THEME §5 "enlarge" action).
 *
 * Replaces the old modal-frame lightbox: the draft opens edge-to-edge on the
 * canvas (--background, THEME §2 "canvas base") with a view-space dot grid,
 * and is inspected losslessly — the bitmap keeps its natural pixel size and
 * is only moved/scaled as a whole, so 100% is true 1:1 and zoom reaches 32×
 * (nearest-neighbor ≥3× for pixel-level checking). Transparency shows as a
 * checkerboard, never a flat color.
 *
 * Interactions: drag to pan (pointer capture), wheel/pinch zooms at the
 * cursor, double-click toggles fit ↔ 1:1, keys Esc/+/-/0/1, floating toolbar
 * with live zoom level. Geometry (background-image on an explicitly sized
 * box) follows ArtImage's WKWebView asset-protocol notes: the bitmap resolves
 * against its own box, and the load probe attaches before src so cached
 * images settle immediately.
 */
export function Lightbox({ open, onClose, imageUrl, filter, alt, caption }: LightboxProps) {
  const { t } = useTranslation();
  const viewportRef = useRef<HTMLDivElement>(null);
  const [viewport, setViewport] = useState({ w: 0, h: 0 });
  const [natural, setNatural] = useState<{ w: number; h: number } | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [view, setView] = useState<View>({ scale: 1, x: 0, y: 0 });
  const [panning, setPanning] = useState(false);
  const viewRef = useRef(view);
  const panRef = useRef<{ pointerId: number; lastX: number; lastY: number } | null>(null);
  const fittedRef = useRef(false);

  const applyView = useCallback((next: View) => {
    viewRef.current = next;
    setView(next);
  }, []);

  // Reset per open/image and probe natural size (handlers before src, cached
  // bitmaps settle synchronously — see ArtImage).
  useEffect(() => {
    if (!open) return;
    setNatural(null);
    setLoaded(false);
    fittedRef.current = false;
    const probe = new Image();
    const settle = () => {
      if (probe.naturalWidth > 0 && probe.naturalHeight > 0) {
        setNatural({ w: probe.naturalWidth, h: probe.naturalHeight });
      }
      setLoaded(true);
    };
    probe.addEventListener("load", settle);
    probe.addEventListener("error", settle);
    probe.src = imageUrl;
    if (probe.complete) settle();
    return () => {
      probe.removeEventListener("load", settle);
      probe.removeEventListener("error", settle);
    };
  }, [open, imageUrl]);

  // Track the viewport box (resizes refit nothing; initial fit runs once).
  useEffect(() => {
    const el = viewportRef.current;
    if (!open || !el) return;
    const measure = () => setViewport({ w: el.clientWidth, h: el.clientHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [open]);

  useEffect(() => {
    if (!open || !natural || viewport.w === 0 || fittedRef.current) return;
    fittedRef.current = true;
    applyView(fitView(natural, viewport));
  }, [open, natural, viewport, applyView]);

  const zoomAt = useCallback(
    (cx: number, cy: number, scale: number) => {
      const { scale: current, x, y } = viewRef.current;
      const next = clampScale(scale);
      if (next === current) return;
      applyView({
        scale: next,
        x: cx - ((cx - x) / current) * next,
        y: cy - ((cy - y) / current) * next,
      });
    },
    [applyView],
  );

  const zoomStepAt = useCallback(
    (cx: number, cy: number, direction: 1 | -1) => {
      zoomAt(cx, cy, viewRef.current.scale * (direction > 0 ? ZOOM_STEP : 1 / ZOOM_STEP));
    },
    [zoomAt],
  );

  const centerPoint = useCallback((): [number, number] => {
    const rect = viewportRef.current?.getBoundingClientRect();
    if (rect) return [rect.left + rect.width / 2, rect.top + rect.height / 2];
    return [window.innerWidth / 2, window.innerHeight / 2];
  }, []);

  const fitToViewport = useCallback(() => {
    if (!natural || viewport.w === 0) return;
    applyView(fitView(natural, viewport));
  }, [natural, viewport, applyView]);

  const actualSize = useCallback(() => {
    if (!natural || viewport.w === 0) return;
    applyView({
      scale: 1,
      x: (viewport.w - natural.w) / 2,
      y: (viewport.h - natural.h) / 2,
    });
  }, [natural, viewport, applyView]);

  // Wheel + trackpad pinch (ctrl+wheel) zoom at the cursor; needs a
  // non-passive native listener to preventDefault reliably.
  useEffect(() => {
    const el = viewportRef.current;
    if (!open || !el) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const dy = event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY;
      if (dy === 0) return;
      zoomAt(event.clientX, event.clientY, viewRef.current.scale * Math.exp(-dy * WHEEL_FACTOR));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [open, zoomAt]);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
        return;
      }
      const [cx, cy] = centerPoint();
      if (event.key === "+" || event.key === "=") zoomStepAt(cx, cy, 1);
      else if (event.key === "-" || event.key === "_") zoomStepAt(cx, cy, -1);
      else if (event.key === "0") fitToViewport();
      else if (event.key === "1") actualSize();
    };
    document.addEventListener("keydown", onKeyDown);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.body.style.overflow = "";
    };
  }, [open, onClose, centerPoint, zoomStepAt, fitToViewport, actualSize]);

  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    panRef.current = { pointerId: event.pointerId, lastX: event.clientX, lastY: event.clientY };
    setPanning(true);
  };

  const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    const pan = panRef.current;
    if (!pan || pan.pointerId !== event.pointerId) return;
    const dx = event.clientX - pan.lastX;
    const dy = event.clientY - pan.lastY;
    pan.lastX = event.clientX;
    pan.lastY = event.clientY;
    const { scale, x, y } = viewRef.current;
    applyView({ scale, x: x + dx, y: y + dy });
  };

  const endPan = (event: React.PointerEvent<HTMLDivElement>) => {
    if (panRef.current?.pointerId !== event.pointerId) return;
    panRef.current = null;
    setPanning(false);
  };

  const onDoubleClick = (event: React.MouseEvent<HTMLDivElement>) => {
    const { scale } = viewRef.current;
    if (scale > 0.98 && scale < 1.02) fitToViewport();
    else zoomAt(event.clientX, event.clientY, 1);
  };

  if (!open) return null;

  const imageW = natural ? natural.w * view.scale : 0;
  const imageH = natural ? natural.h * view.scale : 0;
  const smooth = view.scale < PIXELATED_AT;

  return (
    <div
      data-testid="lightbox"
      role="dialog"
      aria-modal="true"
      aria-label={caption ?? alt}
      className="fixed inset-0 z-50 bg-background"
    >
      <div
        ref={viewportRef}
        data-testid="lightbox-canvas"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endPan}
        onPointerCancel={endPan}
        onDoubleClick={onDoubleClick}
        className={cn(
          "absolute inset-0 touch-none select-none",
          panning ? "cursor-grabbing" : "cursor-grab",
        )}
        style={{
          // View-space dot grid: pans with content via background-position,
          // selling the infinite canvas without tracking per-dot geometry.
          backgroundImage: "radial-gradient(var(--border) 1px, transparent 1px)",
          backgroundSize: `${DOT_GRID}px ${DOT_GRID}px`,
          backgroundPosition: `${view.x % DOT_GRID}px ${view.y % DOT_GRID}px`,
        }}
      >
        {loaded && !natural && (
          <p className="absolute inset-0 flex items-center justify-center text-sm text-muted-foreground">
            {t("lightbox.loadFailed")}
          </p>
        )}
        {natural && (
          <div
            className="absolute left-0 top-0"
            style={{ transform: `translate3d(${view.x}px, ${view.y}px, 0)` }}
          >
            <div
              data-testid="lightbox-image"
              role="img"
              aria-label={alt}
              title={alt}
              className={cn(
                "border bg-card shadow-[0_8px_24px_rgba(2,8,23,0.08)]",
                !loaded && "animate-pulse",
              )}
              style={{
                width: imageW,
                height: imageH,
                // Checkerboard reveals bitmap transparency losslessly.
                backgroundImage:
                  "conic-gradient(var(--muted) 90deg, var(--background) 90deg 180deg, var(--muted) 180deg 270deg, var(--background) 270deg)",
                backgroundSize: "16px 16px",
              }}
            />
            {loaded && (
              <div
                className="absolute inset-0"
                style={{
                  backgroundImage: `url("${imageUrl}")`,
                  backgroundSize: "100% 100%",
                  imageRendering: smooth ? "auto" : "pixelated",
                  ...(filter && filter !== "none" ? { filter } : {}),
                }}
              />
            )}
          </div>
        )}
      </div>

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
            disabled={!natural}
            onClick={() => {
              const [cx, cy] = centerPoint();
              zoomStepAt(cx, cy, -1);
            }}
          >
            <Minus className="size-4" />
          </Button>
          <button
            type="button"
            data-testid="lightbox-zoom-level"
            title={t("lightbox.actualSize")}
            disabled={!natural}
            onClick={actualSize}
            className="min-w-14 rounded-sm px-1 py-1 text-center font-mono text-xs text-muted-foreground outline-none transition-colors duration-150 ease-out hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50"
          >
            {natural ? formatPercent(view.scale) : "–"}
          </button>
          <Button
            variant="ghost"
            size="icon"
            aria-label={t("lightbox.zoomIn")}
            data-testid="lightbox-zoom-in"
            disabled={!natural}
            onClick={() => {
              const [cx, cy] = centerPoint();
              zoomStepAt(cx, cy, 1);
            }}
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
            disabled={!natural}
            onClick={fitToViewport}
          >
            <Scan className="size-4" />
          </Button>
          <Button
            variant="ghost"
            aria-label={t("lightbox.actualSize")}
            title={t("lightbox.actualSize")}
            data-testid="lightbox-actual"
            disabled={!natural}
            onClick={actualSize}
            className="px-2 font-mono text-xs"
          >
            1:1
          </Button>
        </div>
      </footer>
    </div>
  );
}
