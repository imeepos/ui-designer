import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

import { cn } from "@/lib/utils";

/** Imperative canvas controls handed to the owner via the render prop. */
export type CanvasActions = {
  zoomIn: () => void;
  zoomOut: () => void;
  fit: () => void;
  actualSize: () => void;
};

export type InfiniteCanvasProps = {
  imageUrl: string;
  filter?: string;
  alt: string;
  /** Zoom bounds, as fractions (0.01 = 1%, 64 = 6400%). */
  minScale: number;
  maxScale: number;
  /**
   * Double-click semantics: `"fit"` refits the window (embedded stage);
   * `"toggleFitActual"` toggles fit ↔ 1:1 (fullscreen lightbox).
   */
  doubleClick?: "fit" | "toggleFitActual";
  /** Zoom ≥ this renders nearest-neighbor for pixel inspection (lightbox). */
  pixelatedAt?: number;
  /** Show the transparency checkerboard behind the bitmap (lightbox). */
  checkerboard?: boolean;
  /** Keyboard zoom keys: +/- step, 0 fit, 1 actual; Escape calls onEscape. */
  onEscape?: () => void;
  className?: string;
  canvasTestId?: string;
  imageTestId?: string;
  /** Copy shown when the bitmap fails to load (owner-supplied, i18n'd). */
  loadFailedText?: string;
  /**
   * Render-prop overlay for owner chrome (toolbars, zoom badge). Called on
   * every view change with the live zoom percent and imperative actions.
   */
  children?: (api: { percent: string; loaded: boolean; actions: CanvasActions }) => ReactNode;
};

type View = { scale: number; x: number; y: number };
type Size = { w: number; h: number };

const ZOOM_STEP = 1.2;
const WHEEL_FACTOR = 0.0015;
const FIT_MARGIN = 64;
const DOT_GRID = 24;

/**
 * Shared infinite-canvas surface (extracted from the Phase lightbox rework):
 * wheel/pinch zooms at the cursor, drag pans via pointer capture, and the
 * bitmap only ever moves/scales as a whole (`translate3d` + explicit box +
 * background-image), so panning/zooming never re-renders the image itself.
 *
 * The dot grid pans with the content via background-position, selling the
 * infinite canvas without tracking per-dot geometry. All owner chrome —
 * toolbars, zoom badges, dialogs — is composed through the render prop so
 * this component stays pure geometry (one place to fix bugs, two places to
 * style: fullscreen lightbox and the embedded workspace stage).
 */
export function InfiniteCanvas({
  imageUrl,
  filter,
  alt,
  minScale,
  maxScale,
  doubleClick = "fit",
  pixelatedAt,
  checkerboard = false,
  onEscape,
  className,
  canvasTestId,
  imageTestId,
  loadFailedText,
  children,
}: InfiniteCanvasProps) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const [viewport, setViewport] = useState<Size>({ w: 0, h: 0 });
  const [natural, setNatural] = useState<Size | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [view, setView] = useState<View>({ scale: 1, x: 0, y: 0 });
  const [panning, setPanning] = useState(false);
  const viewRef = useRef(view);
  const panRef = useRef<{ pointerId: number; lastX: number; lastY: number } | null>(null);
  const fittedRef = useRef(false);
  // Bounds in a ref so zoom handlers read the latest values without
  // re-subscribing the wheel listener on every prop change.
  const boundsRef = useRef({ minScale, maxScale });
  boundsRef.current = { minScale, maxScale };

  const clampScale = useCallback(
    (scale: number) => {
      const { minScale: min, maxScale: max } = boundsRef.current;
      return Math.min(max, Math.max(min, scale));
    },
    [],
  );

  const applyView = useCallback((next: View) => {
    viewRef.current = next;
    setView(next);
  }, []);

  // Probe natural size (handlers attached before src so cached bitmaps
  // settle synchronously — see ArtImage's WKWebView notes).
  useEffect(() => {
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
  }, [imageUrl]);

  // Track the viewport box (resizes refit nothing; the initial fit runs once).
  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    const measure = () => setViewport({ w: el.clientWidth, h: el.clientHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const fitView = useCallback(
    (naturalSize: Size, viewportSize: Size): View => {
      const scale = clampScale(
        Math.min(
          (viewportSize.w - FIT_MARGIN) / naturalSize.w,
          (viewportSize.h - FIT_MARGIN) / naturalSize.h,
        ),
      );
      return {
        scale,
        x: (viewportSize.w - naturalSize.w * scale) / 2,
        y: (viewportSize.h - naturalSize.h * scale) / 2,
      };
    },
    [clampScale],
  );

  useEffect(() => {
    if (!natural || viewport.w === 0 || fittedRef.current) return;
    fittedRef.current = true;
    applyView(fitView(natural, viewport));
  }, [natural, viewport, fitView, applyView]);

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
    [applyView, clampScale],
  );

  const centerPoint = useCallback((): [number, number] => {
    const rect = viewportRef.current?.getBoundingClientRect();
    if (rect) return [rect.left + rect.width / 2, rect.top + rect.height / 2];
    return [window.innerWidth / 2, window.innerHeight / 2];
  }, []);

  const fitToViewport = useCallback(() => {
    if (!natural || viewport.w === 0) return;
    applyView(fitView(natural, viewport));
  }, [natural, viewport, fitView, applyView]);

  const actualSize = useCallback(() => {
    if (!natural || viewport.w === 0) return;
    applyView({
      scale: 1,
      x: (viewport.w - natural.w) / 2,
      y: (viewport.h - natural.h) / 2,
    });
  }, [natural, viewport, applyView]);

  const actions = useMemo<CanvasActions>(
    () => ({
      zoomIn: () => {
        const [cx, cy] = centerPoint();
        zoomAt(cx, cy, viewRef.current.scale * ZOOM_STEP);
      },
      zoomOut: () => {
        const [cx, cy] = centerPoint();
        zoomAt(cx, cy, viewRef.current.scale / ZOOM_STEP);
      },
      fit: fitToViewport,
      actualSize,
    }),
    [centerPoint, zoomAt, fitToViewport, actualSize],
  );

  // Wheel + trackpad pinch (ctrl+wheel) zoom at the cursor; needs a
  // non-passive native listener to preventDefault reliably.
  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const dy = event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY;
      if (dy === 0) return;
      zoomAt(event.clientX, event.clientY, viewRef.current.scale * Math.exp(-dy * WHEEL_FACTOR));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [zoomAt]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onEscape?.();
        return;
      }
      if (event.key === "+" || event.key === "=") actions.zoomIn();
      else if (event.key === "-" || event.key === "_") actions.zoomOut();
      else if (event.key === "0") actions.fit();
      else if (event.key === "1") actions.actualSize();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [actions, onEscape]);

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
    if (doubleClick === "fit") {
      fitToViewport();
      return;
    }
    const { scale } = viewRef.current;
    if (scale > 0.98 && scale < 1.02) fitToViewport();
    else zoomAt(event.clientX, event.clientY, 1);
  };

  const imageW = natural ? natural.w * view.scale : 0;
  const imageH = natural ? natural.h * view.scale : 0;
  const percent = formatPercent(view.scale);
  const pixelated = pixelatedAt !== undefined && view.scale >= pixelatedAt;

  return (
    <div className={cn("relative", className)}>
      {/*
        The interactive viewport is a child of (not a wrapper around) the
        owner chrome, so toolbar clicks never start a pan gesture and stay
        crisp above the canvas.
      */}
      <div
        ref={viewportRef}
        data-testid={canvasTestId}
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
        {loaded && !natural && loadFailedText && (
          <p className="absolute inset-0 flex items-center justify-center text-sm text-muted-foreground">
            {loadFailedText}
          </p>
        )}
        {natural && (
          <div
            className="absolute left-0 top-0"
            style={{ transform: `translate3d(${view.x}px, ${view.y}px, 0)` }}
          >
            <div
              data-testid={imageTestId}
              role="img"
              aria-label={alt}
              title={alt}
              className={cn(
                "border shadow-[0_8px_24px_rgba(2,8,23,0.08)]",
                !loaded && "animate-pulse",
              )}
              style={{
                width: imageW,
                height: imageH,
                ...(checkerboard
                  ? {
                      // Checkerboard reveals bitmap transparency losslessly.
                      backgroundImage:
                        "conic-gradient(var(--muted) 90deg, var(--background) 90deg 180deg, var(--muted) 180deg 270deg, var(--background) 270deg)",
                      backgroundSize: "16px 16px",
                    }
                  : { backgroundColor: "var(--card)" }),
              }}
            />
            {loaded && (
              <div
                className="absolute inset-0"
                style={{
                  backgroundImage: `url("${imageUrl}")`,
                  backgroundSize: "100% 100%",
                  imageRendering: pixelated ? "pixelated" : "auto",
                  ...(filter && filter !== "none" ? { filter } : {}),
                }}
              />
            )}
          </div>
        )}
      </div>
      {children?.({ percent, loaded, actions })}
    </div>
  );
}

const formatPercent = (scale: number): string => {
  const percent = scale * 100;
  return `${percent >= 10 ? Math.round(percent) : percent.toFixed(1)}%`;
};
