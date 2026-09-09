// Slug + text validation shared by the page/component forms.
export const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
export const SLUG_MAX_LENGTH = 48;
export const NAME_MAX_LENGTH = 64;

export function isValidSlug(slug: string): boolean {
  return slug.length > 0 && slug.length <= SLUG_MAX_LENGTH && SLUG_PATTERN.test(slug);
}

export function slugifyHint(input: string): string {
  return input
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, SLUG_MAX_LENGTH);
}

/**
 * Shared required → taken → format check for page slugs and component names
 * (both hit core `store::validate_identifier`, so both use the slug rule).
 * Returns the i18n error key, or `null` when the value may be submitted.
 */
export function identifierErrorKey(
  value: string,
  taken: boolean,
  keys: { required: string; taken: string; format: string },
): string | null {
  const trimmed = value.trim();
  if (!trimmed) return keys.required;
  if (taken) return keys.taken;
  if (!isValidSlug(trimmed)) return keys.format;
  return null;
}
