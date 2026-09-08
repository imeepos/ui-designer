import { useTranslation } from "react-i18next";

import { persistLanguage } from "@/i18n";
import { cn } from "@/lib/utils";

export const SUPPORTED_LANGUAGES = ["zh-CN", "en"] as const;
export type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number];

export function toSupportedLanguage(language: string): SupportedLanguage {
  return language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export function LanguageSwitcher({ className }: { className?: string }) {
  const { t, i18n } = useTranslation();
  const current = toSupportedLanguage(i18n.language);

  return (
    <div
      role="group"
      aria-label={t("language.label")}
      data-testid="language-switcher"
      className={cn("flex items-center gap-0.5 rounded-md border p-0.5", className)}
    >
      {SUPPORTED_LANGUAGES.map((lang) => (
        <button
          key={lang}
          type="button"
          aria-pressed={current === lang}
          onClick={() => {
            persistLanguage(lang);
            void i18n.changeLanguage(lang);
          }}
          className={cn(
            "rounded-sm px-2 py-0.5 text-xs transition-colors duration-150 ease-out",
            current === lang
              ? "bg-secondary font-medium text-secondary-foreground"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {t(`language.${lang}`)}
        </button>
      ))}
    </div>
  );
}
