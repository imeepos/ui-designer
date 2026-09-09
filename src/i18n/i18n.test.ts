import { beforeAll, describe, expect, test } from "vitest";

import i18n from "./index";

beforeAll(async () => {
  await i18n.changeLanguage("zh-CN");
});

describe("VALIDATION_ERROR detail passthrough", () => {
  test("zh-CN renders the backend detail via {{detail}}", () => {
    const text = i18n.t("errors.VALIDATION_ERROR.message", {
      detail: "invalid component name `主按钮`: use lowercase a-z, 0-9 and '-'",
    });
    expect(text).toBe(
      "输入不合法：invalid component name `主按钮`: use lowercase a-z, 0-9 and '-'",
    );
  });

  test("en renders the backend detail via {{detail}}", async () => {
    await i18n.changeLanguage("en");
    const text = i18n.t("errors.VALIDATION_ERROR.message", {
      detail: "component brief must not be empty",
    });
    expect(text).toBe("Invalid input: component brief must not be empty");
    await i18n.changeLanguage("zh-CN");
  });
});

describe("form error keys exist in every supported language", () => {
  const KEYS = [
    "form.error.slugRequired",
    "form.error.slugTaken",
    "form.error.slugFormat",
    "form.error.nameRequired",
    "form.error.nameTaken",
    "form.error.nameFormat",
    "form.error.briefRequired",
    "form.component.nameHint",
  ];

  for (const language of ["zh-CN", "en"] as const) {
    test(language, async () => {
      await i18n.changeLanguage(language);
      for (const key of KEYS) {
        const text = i18n.t(key);
        expect(text, `${language}:${key} missing`).not.toBe(key);
        expect(text.length).toBeGreaterThan(0);
      }
      await i18n.changeLanguage("zh-CN");
    });
  }
});
