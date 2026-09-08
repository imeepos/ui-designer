// Mock fixture images + draft variants.
// The fixture PNGs live in src/mocks/fixtures/ (copied from the smoke run).
// Candidate drafts are simulated with CSS filter variants of the same fixture.
import boardFixture from "@/mocks/fixtures/board.png";
import pageFixture from "@/mocks/fixtures/page.png";

export const BOARD_FIXTURE = boardFixture;
export const PAGE_FIXTURE = pageFixture;

/** Deterministic filter variants so each candidate looks like its own draft. */
const VARIANTS: string[] = [
  "none",
  "hue-rotate(14deg) saturate(1.08)",
  "hue-rotate(-12deg) brightness(1.04)",
  "saturate(0.85) contrast(1.05)",
  "hue-rotate(28deg) brightness(0.97) saturate(1.05)",
];

export function variantFilter(index: number): string | undefined {
  const filter = VARIANTS[index % VARIANTS.length];
  return filter === "none" ? undefined : filter;
}
