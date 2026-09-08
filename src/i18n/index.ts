import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./en.json";
import zhCN from "./zh-CN.json";

export const SUPPORTED_LANGUAGES = ["zh-CN", "en"] as const;
export type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number];

void i18n.use(initReactI18next).init({
  resources: {
    "zh-CN": { translation: zhCN },
    en: { translation: en },
  },
  lng: "zh-CN",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
  react: { useSuspense: false },
});

export default i18n;
