import { useEffect, useRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

type ModalProps = {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  footer?: ReactNode;
  /** Wider shell for galleries inside modals. */
  wide?: boolean;
  testId?: string;
};

/** Minimal modal shell: overlay + card, Escape closes, focus moves inside. */
export function Modal({ open, onClose, title, children, footer, wide, testId }: ModalProps) {
  const { t } = useTranslation();
  const cardRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKeyDown);
    const focusable = cardRef.current?.querySelector<HTMLElement>(
      "input, textarea, select, button",
    );
    focusable?.focus();
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.body.style.overflow = "";
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      data-testid={testId}
      className="fixed inset-0 z-40 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
      aria-label={title}
    >
      <button
        type="button"
        aria-label={t("common.close")}
        tabIndex={-1}
        onClick={onClose}
        className="absolute inset-0 bg-foreground/25"
      />
      <div
        ref={cardRef}
        className={cn(
          "relative flex max-h-[85vh] w-full flex-col rounded-xl border bg-card shadow-[0_8px_24px_rgba(2,8,23,0.08)]",
          wide ? "max-w-3xl" : "max-w-md",
        )}
      >
        <div className="flex h-11 shrink-0 items-center justify-between border-b px-4">
          <h2 className="text-sm font-semibold">{title}</h2>
          <Button variant="ghost" size="icon" aria-label={t("common.close")} onClick={onClose}>
            <X className="size-4" />
          </Button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto p-4">{children}</div>
        {footer && <div className="flex shrink-0 items-center justify-end gap-2 border-t px-4 py-3">{footer}</div>}
      </div>
    </div>
  );
}
