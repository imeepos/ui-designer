import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

type ArtImageProps = {
  src: string;
  /** Mock draft variant; applied as CSS filter (mock adapter only). */
  filter?: string;
  alt: string;
  className?: string;
  imgClassName?: string;
  testId?: string;
};

/**
 * Fixture/candidate image with a skeleton placeholder while decoding.
 * The mock uses one fixture per artifact kind; `filter` distinguishes drafts.
 */
export function ArtImage({ src, filter, alt, className, imgClassName, testId }: ArtImageProps) {
  const [loaded, setLoaded] = useState(false);

  // Reset when the mock swaps fixtures underneath (project switch).
  useEffect(() => {
    setLoaded(false);
  }, [src]);

  return (
    <div
      data-testid={testId}
      className={cn(
        "relative overflow-hidden bg-muted",
        !loaded && "animate-pulse",
        className,
      )}
    >
      <img
        src={src}
        alt={alt}
        draggable={false}
        loading="lazy"
        onLoad={() => setLoaded(true)}
        style={filter && filter !== "none" ? { filter } : undefined}
        className={cn(
          "absolute inset-0 size-full object-cover transition-opacity duration-150 ease-out",
          loaded ? "opacity-100" : "opacity-0",
          imgClassName,
        )}
      />
    </div>
  );
}
