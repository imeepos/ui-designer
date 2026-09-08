import { Maximize2, Anchor, Trash2, RefreshCw, Check } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { ReactNode } from "react";

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

/** Icon button used inside ArtifactCard hover actions; ghost posture. */
export function CardAction({
  label,
  onClick,
  children,
  destructive,
  primary,
  testId,
}: {
  label: string;
  onClick: () => void;
  children: ReactNode;
  destructive?: boolean;
  primary?: boolean;
  testId?: string;
}) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      aria-label={label}
      title={label}
      data-testid={testId}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
      className={cn(
        "size-6 rounded-sm border border-border bg-card/95 text-muted-foreground shadow-none transition-colors duration-150 ease-out hover:border-primary hover:text-foreground",
        destructive && "hover:border-destructive/50 hover:text-destructive",
        primary && "text-primary hover:text-primary",
      )}
    >
      {children}
    </Button>
  );
}

/** Hover-overlay glyphs are linear 1.5px (THEME §5 v2 posture). */
export function EnlargeIcon({ className }: { className?: string }) {
  return <Maximize2 strokeWidth={1.5} className={cn("size-3", className)} />;
}

export function AnchorIcon({ className }: { className?: string }) {
  return <Anchor strokeWidth={1.5} className={cn("size-3", className)} />;
}

export function TrashIcon({ className }: { className?: string }) {
  return <Trash2 strokeWidth={1.5} className={cn("size-3", className)} />;
}

export function RegenerateIcon({ className }: { className?: string }) {
  return <RefreshCw strokeWidth={1.5} className={cn("size-3", className)} />;
}

export function PickIcon({ className }: { className?: string }) {
  return <Check strokeWidth={1.5} className={cn("size-3", className)} />;
}

/** Anchor mark: brass badge with helm glyph (THEME §5, accent token). */
export function AnchorBadge({ className }: { className?: string }) {
  const { t } = useTranslation();
  return (
    <span
      data-testid="anchor-badge"
      className={cn(
        "inline-flex items-center gap-1 rounded-full bg-accent px-2 py-0.5 text-[11px] font-semibold text-accent-foreground",
        className,
      )}
    >
      <Anchor className="size-3" aria-hidden="true" />
      {t("gallery.board.anchorBadge")}
    </span>
  );
}
