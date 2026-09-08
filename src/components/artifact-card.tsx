import type { ReactNode } from "react";

import { ArtImage } from "@/components/art-image";
import { cn } from "@/lib/utils";

type ArtifactCardProps = {
  imageUrl: string;
  filter?: string;
  alt: string;
  /** Badge rendered top-left over the image (e.g. anchor mark). */
  badge?: ReactNode;
  /** Ghost actions floating top-right on hover (THEME §5). */
  actions?: ReactNode;
  footer?: ReactNode;
  /** Rendered instead of the image when `imageUrl` is empty. */
  placeholder?: ReactNode;
  aspect?: "square" | "portrait" | "video";
  selected?: boolean;
  onClick?: () => void;
  testId?: string;
};

const ASPECT_CLASSES: Record<NonNullable<ArtifactCardProps["aspect"]>, string> = {
  square: "aspect-square",
  portrait: "aspect-[3/4]",
  video: "aspect-video",
};

/**
 * Gallery card posture (THEME §5): 1px border, primary outline + floating
 * ghost actions on hover, 12px radius.
 */
export function ArtifactCard({
  imageUrl,
  filter,
  alt,
  badge,
  actions,
  footer,
  placeholder,
  aspect = "square",
  selected,
  onClick,
  testId,
}: ArtifactCardProps) {
  return (
    <figure
      data-testid={testId}
      className={cn(
        "group relative flex flex-col overflow-hidden rounded-lg border bg-card transition-[border-color,box-shadow] duration-150 ease-out",
        onClick && "cursor-pointer",
        selected
          ? "border-primary ring-1 ring-primary"
          : "hover:border-primary hover:shadow-[0_0_0_1px_var(--primary)]",
      )}
      onClick={onClick}
    >
      <div className="relative">
        {imageUrl ? (
          <ArtImage
            src={imageUrl}
            filter={filter}
            alt={alt}
            className={cn("w-full", ASPECT_CLASSES[aspect])}
          />
        ) : (
          <div className={cn("w-full bg-muted/40", ASPECT_CLASSES[aspect])}>{placeholder}</div>
        )}
        {badge && <div className="absolute top-1.5 left-1.5">{badge}</div>}
        {actions && (
          <div className="absolute top-1.5 right-1.5 flex gap-1 rounded-md p-0.5 opacity-0 shadow-[0_8px_24px_rgba(2,8,23,0.08)] transition-opacity duration-150 ease-out group-hover:opacity-100 group-focus-within:opacity-100">
            {actions}
          </div>
        )}
      </div>
      {footer && (
        <figcaption className="flex items-center gap-2 border-t px-2.5 py-1.5 text-[11px] text-muted-foreground">
          {footer}
        </figcaption>
      )}
    </figure>
  );
}
