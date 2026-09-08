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

/** Compass: empty-state illustration */
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
