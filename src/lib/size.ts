import type { CanvasSize, SizePreset } from "@/lib/api/types";

// gpt-image-2 constraints (PRD 3.1): sides multiple of 16, aspect ratio <= 3:1,
// total pixels between 0.65MP and 8.3MP.
export const SIZE_MIN_PIXELS = 650_000;
export const SIZE_MAX_PIXELS = 8_300_000;
export const SIZE_MULTIPLE = 16;
export const SIZE_MAX_ASPECT = 3;

export const SIZE_PRESETS: Record<Exclude<SizePreset, "custom">, CanvasSize> = {
  web: { w: 1536, h: 1024, preset: "web" },
  mobile: { w: 1024, h: 1536, preset: "mobile" },
  desktop: { w: 2560, h: 1440, preset: "desktop" },
};

export type SizeValidation =
  | { ok: true; size: CanvasSize }
  | { ok: false; code: "required" | "multiple" | "aspect" | "pixels" | "range"; params?: Record<string, string | number> };

function toInt(value: string): number | null {
  const trimmed = value.trim();
  if (!/^\d+$/.test(trimmed)) return null;
  return Number.parseInt(trimmed, 10);
}

/**
 * Validate a canvas size. `w`/`h` accept raw form strings; preset sizes are
 * always valid by construction but still checked for defence in depth.
 */
export function validateCanvasSize(
  preset: SizePreset,
  wRaw: string,
  hRaw: string,
): SizeValidation {
  let w: number | null;
  let h: number | null;
  if (preset === "custom") {
    w = toInt(wRaw);
    h = toInt(hRaw);
    if (w === null || h === null || w <= 0 || h <= 0) {
      return { ok: false, code: "required" };
    }
  } else {
    const fixed = SIZE_PRESETS[preset];
    w = fixed.w;
    h = fixed.h;
  }

  if (w % SIZE_MULTIPLE !== 0 || h % SIZE_MULTIPLE !== 0) {
    return {
      ok: false,
      code: "multiple",
      params: { multiple: SIZE_MULTIPLE, w: w % SIZE_MULTIPLE, h: h % SIZE_MULTIPLE },
    };
  }

  const long = Math.max(w, h);
  const short = Math.min(w, h);
  if (long / short > SIZE_MAX_ASPECT) {
    return { ok: false, code: "aspect", params: { max: SIZE_MAX_ASPECT } };
  }

  const pixels = w * h;
  if (pixels < SIZE_MIN_PIXELS || pixels > SIZE_MAX_PIXELS) {
    return {
      ok: false,
      code: "pixels",
      params: { pixels: (pixels / 1_000_000).toFixed(2), min: "0.65", max: "8.3" },
    };
  }

  return { ok: true, size: { w, h, preset } };
}

export function formatSize(size: CanvasSize): string {
  return `${size.w}x${size.h}`;
}
