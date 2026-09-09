import { cn } from "@/lib/utils";

// Nautical line-art icons (inline SVG only, no external assets)
type IconProps = {
  className?: string;
};

/** Ship helm: app mark */
export function HelmMark({ className }: IconProps) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      aria-hidden="true"
      className={cn("size-5", className)}
    >
      <circle cx="12" cy="12" r="7" />
      <circle cx="12" cy="12" r="2.5" />
      <path d="M19 12h3M5 12H2M12 5V2M12 19v3" />
      <path d="M16.95 7.05l2.12-2.12M7.05 7.05L4.93 4.93M16.95 16.95l2.12 2.12M7.05 16.95l-2.12 2.12" />
    </svg>
  );
}

/** Compass: empty-state illustration (THEME §5: nautical line art, 1.5px stroke). */
export function CompassMark({ className }: IconProps) {
  return (
    <svg
      viewBox="0 0 96 96"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={cn("size-24", className)}
    >
      <circle cx="48" cy="48" r="34" />
      <circle cx="48" cy="48" r="27" strokeDasharray="2 4" />
      <path d="M48 14v-6M48 88v-6M14 48H8M88 48h-6" />
      <path d="M58 38l-6.5 14.5L38 58l6.5-14.5L58 38z" />
      <circle cx="48" cy="48" r="1.5" />
    </svg>
  );
}

/** Anchor: empty-state illustration (THEME §5 v2: helm/compass/anchor only). */
export function AnchorMark({ className }: IconProps) {
  return (
    <svg
      viewBox="0 0 96 96"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={cn("size-24", className)}
    >
      <circle cx="48" cy="22" r="8" />
      <path d="M48 30v52" />
      <path d="M32 42h32" />
      <path d="M20 58c0 16 12.5 24 28 24s28-8 28-24" />
      <path d="M20 58l-6-4M20 58l7-2M76 58l6-4M76 58l-7-2" />
    </svg>
  );
}

/**
 * Sailing boat: empty-state illustration (THEME §5 v3: helm/compass/anchor/boat).
 * Baimiao (ink line-art) — hull, mast, two sails, calm water and two gulls,
 * echoing the MoHang anchor board (design-china/board/anchor.png).
 */
export function BoatMark({ className }: IconProps) {
  return (
    <svg
      viewBox="0 0 96 96"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={cn("size-24", className)}
    >
      {/* mast */}
      <path d="M48 18v48" />
      {/* main sail */}
      <path d="M52 22c11 8 16 20 17 34l-17 2" />
      {/* jib sail */}
      <path d="M44 28c-8 6-12 14-13 24h13" />
      {/* hull */}
      <path d="M22 68h52c-3 5-9 8-15 8H37c-6 0-12-3-15-8z" />
      {/* calm water */}
      <path d="M12 82c6-4 12-4 18 0s12 4 18 0 12-4 18 0 12 4 18 0" />
      {/* gulls */}
      <path d="M66 24c2-2.5 4.5-2.5 6.5 0M74 30c2-2.5 4.5-2.5 6.5 0" />
    </svg>
  );
}
