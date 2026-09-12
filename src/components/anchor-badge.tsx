import { Anchor } from "lucide-react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";

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
