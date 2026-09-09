import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

type ArtImageProps = {
  src: string;
  /** Mock draft variant; applied as CSS filter (mock adapter only). */
  filter?: string;
  alt: string;
  className?: string;
  /** How the bitmap fits the box: cover (default) or contain (lightbox). */
  fit?: "cover" | "contain";
  testId?: string;
};

/**
 * Fixture/candidate image with a skeleton placeholder while decoding.
 * The mock uses one fixture per artifact kind; `filter` distinguishes drafts.
 *
 * Rendered as a background-image div instead of <img> + object-fit. The
 * desktop WKWebView (Tauri shell) has two asset-protocol image defects:
 * 1) local loads can finish before React attaches the onLoad listener, so
 *    `loaded` never flips and the bitmap stays at opacity-0 (invisible);
 * 2) inside containers whose height derives from an ancestor aspect-ratio,
 *    object-cover rasterizes the bitmap against the wrong box, so previews
 *    show only a magnified corner of the artwork.
 * background-size resolves against the element's own box and hits neither
 * path, keeping rendering identical across engines. Readiness still drives
 * the animate-pulse skeleton via a JS Image probe (handlers attached before
 * src, so cached bitmaps settle immediately).
 */
export function ArtImage({ src, filter, alt, className, fit = "cover", testId }: ArtImageProps) {
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    setLoaded(false);
    const probe = new Image();
    const settle = () => setLoaded(true);
    probe.addEventListener("load", settle);
    probe.addEventListener("error", settle);
    probe.src = src;
    // Cached bitmaps may complete synchronously; settle without the event too.
    if (probe.complete) settle();
    return () => {
      probe.removeEventListener("load", settle);
      probe.removeEventListener("error", settle);
    };
  }, [src]);

  return (
    <div
      role="img"
      aria-label={alt}
      title={alt}
      data-testid={testId}
      style={{
        backgroundImage: `url("${src}")`,
        backgroundSize: fit,
        ...(filter && filter !== "none" ? { filter } : {}),
      }}
      className={cn(
        "relative overflow-hidden bg-muted bg-center bg-no-repeat",
        !loaded && "animate-pulse",
        className,
      )}
    />
  );
}
