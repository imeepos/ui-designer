import type { ReactNode } from "react";

import { AnchorMark, CompassMark, HelmMark } from "@/components/icons";
import { cn } from "@/lib/utils";

/** Allowed nautical marks only (THEME §5 v2): helm / compass / anchor. */
type EmptyStateMark = "helm" | "compass" | "anchor";

const MARKS = {
  helm: HelmMark,
  compass: CompassMark,
  anchor: AnchorMark,
} as const;

type EmptyStateProps = {
  title: string;
  description?: string;
  action?: ReactNode;
  /** Nautical line-art mark; defaults to the compass. */
  mark?: EmptyStateMark;
  className?: string;
};

/**
 * Empty state posture (THEME §5 v2): centered 176px line illustration
 * (within the 160-200px band), 1.5px stroke, single navy tone
 * (`--primary`), 24px gap below the artwork, then title/CTA.
 */
export function EmptyState({ title, description, action, mark = "compass", className }: EmptyStateProps) {
  const Mark = MARKS[mark];
  return (
    <div
      data-testid="empty-state"
      className={cn("flex flex-1 flex-col items-center justify-center p-6", className)}
    >
      <Mark className="mb-6 size-[176px] text-primary" />
      <div className="flex flex-col items-center gap-1 text-center">
        <p className="text-lg font-medium text-foreground">{title}</p>
        {description && (
          <p className="max-w-72 text-xs text-muted-foreground">{description}</p>
        )}
      </div>
      {action && <div className="mt-3">{action}</div>}
    </div>
  );
}
