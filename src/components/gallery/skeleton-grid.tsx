import { cn } from "@/lib/utils";

/** THEME §5 generation skeleton: muted pulse blocks sized like artifact cards. */
export function SkeletonCard({ aspect = "square" }: { aspect?: "square" | "video" }) {
  return (
    <div
      data-testid="skeleton-card"
      className={cn(
        "animate-pulse rounded-lg border bg-muted",
        aspect === "square" ? "aspect-square" : "aspect-video",
      )}
    />
  );
}

export function SkeletonGrid({
  count,
  aspect = "square",
}: {
  count: number;
  aspect?: "square" | "video";
}) {
  return (
    <>
      {Array.from({ length: count }, (_, index) => (
        <SkeletonCard key={index} aspect={aspect} />
      ))}
    </>
  );
}
