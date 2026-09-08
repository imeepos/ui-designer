import { CheckCircle2, Info, X, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import type { ToastItem } from "@/state/toast";

type ToastViewportProps = {
  items: ToastItem[];
  onDismiss: (id: number) => void;
};

/** Fixed viewport, bottom-right; THEME: 1px border, card face, no heavy shadow. */
export function ToastViewport({ items, onDismiss }: ToastViewportProps) {
  if (items.length === 0) return null;
  return (
    <div
      data-testid="toast-viewport"
      role="status"
      aria-live="polite"
      className="pointer-events-none fixed right-4 bottom-4 z-50 flex w-80 flex-col gap-2"
    >
      {items.map((item) => (
        <ToastCard key={item.id} item={item} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function ToastCard({
  item,
  onDismiss,
}: {
  item: ToastItem;
  onDismiss: (id: number) => void;
}) {
  const { t } = useTranslation();
  return (
    <div
      data-testid={`toast-${item.variant}`}
      className={cn(
        "pointer-events-auto flex items-start gap-2.5 rounded-lg border bg-card p-3 shadow-[0_8px_24px_rgba(2,8,23,0.08)]",
        item.variant === "error" && "border-destructive/40",
      )}
    >
      <span
        aria-hidden="true"
        className={cn(
          "mt-0.5 shrink-0",
          item.variant === "success" && "text-primary",
          item.variant === "info" && "text-muted-foreground",
          item.variant === "error" && "text-destructive",
        )}
      >
        {item.variant === "success" ? (
          <CheckCircle2 className="size-4" />
        ) : item.variant === "error" ? (
          <XCircle className="size-4" />
        ) : (
          <Info className="size-4" />
        )}
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        {item.code && (
          <span className="font-mono text-[11px] tracking-wide text-muted-foreground uppercase">
            {item.code}
          </span>
        )}
        <p className="text-xs leading-snug break-words text-foreground">{item.message}</p>
        {item.hint && (
          <p className="text-xs leading-snug break-words text-muted-foreground">{item.hint}</p>
        )}
      </div>
      <button
        type="button"
        data-testid="toast-dismiss"
        aria-label={t("common.close")}
        onClick={() => onDismiss(item.id)}
        className="shrink-0 rounded-sm p-0.5 text-muted-foreground transition-colors duration-150 ease-out hover:text-foreground"
      >
        <X className="size-3.5" />
      </button>
    </div>
  );
}
