import type { ReactNode } from "react";

import { CompassMark } from "@/components/icons";
import { cn } from "@/lib/utils";

type EmptyStateProps = {
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
};

/** Empty state: centered compass line art + primary CTA (THEME §5) */
export function EmptyState({ title, description, action, className }: EmptyStateProps) {
  return (
    <div
      data-testid="empty-state"
      className={cn("flex flex-1 flex-col items-center justify-center gap-3 p-6", className)}
    >
      <CompassMark className="size-24 text-border" />
      <div className="flex flex-col items-center gap-1 text-center">
        <p className="text-lg font-medium text-foreground">{title}</p>
        {description && (
          <p className="max-w-72 text-xs text-muted-foreground">{description}</p>
        )}
      </div>
      {action}
    </div>
  );
}
