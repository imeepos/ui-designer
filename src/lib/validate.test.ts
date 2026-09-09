import { describe, expect, test } from "vitest";

import { identifierErrorKey, isValidSlug, slugifyHint } from "./validate";

const NAME_KEYS = {
  required: "form.error.nameRequired",
  taken: "form.error.nameTaken",
  format: "form.error.nameFormat",
};

describe("identifierErrorKey", () => {
  test("blank value → required", () => {
    expect(identifierErrorKey("", false, NAME_KEYS)).toBe(NAME_KEYS.required);
    expect(identifierErrorKey("   ", false, NAME_KEYS)).toBe(NAME_KEYS.required);
  });

  test("duplicate value → taken (before format check)", () => {
    expect(identifierErrorKey("button-set", true, NAME_KEYS)).toBe(NAME_KEYS.taken);
  });

  test("illegal charset → format", () => {
    for (const bad of ["主按钮", "Button", "btn_main", "-lead", "trail-", "a b"]) {
      expect(identifierErrorKey(bad, false, NAME_KEYS)).toBe(NAME_KEYS.format);
    }
  });

  test("valid value → null", () => {
    expect(identifierErrorKey("button-set", false, NAME_KEYS)).toBeNull();
    expect(identifierErrorKey("  nav2 ", false, NAME_KEYS)).toBeNull();
  });
});

describe("slugifyHint (component name auto-fix on blur)", () => {
  test("normalizes case, spaces and underscores to dashes", () => {
    expect(slugifyHint("My Button_2")).toBe("my-button-2");
    expect(slugifyHint("  Trimmed  ")).toBe("trimmed");
  });

  test("strips leading/trailing dashes and caps length", () => {
    expect(slugifyHint("--a--b--")).toBe("a-b");
    expect(slugifyHint("x".repeat(60)).length).toBe(48);
  });

  test("non-latin input collapses to empty string", () => {
    expect(slugifyHint("中文按钮")).toBe("");
  });

  test("stays compatible with isValidSlug", () => {
    for (const input of ["My Button_2", "--a--b--", "x".repeat(60)]) {
      expect(isValidSlug(slugifyHint(input))).toBe(true);
    }
  });
});
