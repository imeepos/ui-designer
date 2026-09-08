/**
 * Unified detail-form schema (docs/UI-REVIEW.md improvement #4).
 *
 * One source of truth for the fields every detail form renders — identifier,
 * brief (shared length cap + counter), candidate count (1-4) and the quality
 * enum — so the board/page/component panels can never drift apart again.
 * Copy lives in src/i18n/*.json (`form.quality.*`, `form.briefCount`).
 */

/** Max characters for any brief textarea (board brand/page/component). */
export const BRIEF_MAX = 200;

/** Candidate count bounds (PRD: 1-4 drafts per generation). */
export const COUNT_MIN = 1;
export const COUNT_MAX = 4;

/** Quality enum mirroring rudder-core `QUALITY_LEVELS` (default: low). */
export const QUALITY_LEVELS = ["low", "medium", "high"] as const;
export type QualityLevel = (typeof QUALITY_LEVELS)[number];

/** Exploration tier by default (UI-REVIEW defect #5 ruling). */
export const DEFAULT_QUALITY: QualityLevel = "low";
