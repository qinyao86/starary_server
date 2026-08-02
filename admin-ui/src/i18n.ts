import { en } from "./i18n/en";
import { zh } from "./i18n/zh";

export type Language = "en" | "es" | "de" | "ru" | "zh-TW" | "zh-CN" | "ja" | "ko";

export const dictionaries = { en, "zh-CN": zh } as const;

export const languageOptions: Array<{ value: Language; label: string }> = [
  { value: "en", label: "English" },
  { value: "es", label: "Español" },
  { value: "de", label: "Deutsch" },
  { value: "ru", label: "Русский" },
  { value: "zh-TW", label: "繁體中文" },
  { value: "zh-CN", label: "简体中文" },
  { value: "ja", label: "日本語" },
  { value: "ko", label: "한국어" },
];

export type TranslationKey = keyof typeof en;

export function isLanguage(value: string | null): value is Language {
  return languageOptions.some((option) => option.value === value);
}

export function normalizeLanguage(value: string | null): Language {
  if (isLanguage(value)) {
    return value;
  }
  if (value === "zh") {
    return "zh-CN";
  }
  return "zh-CN";
}

export function createTranslator(language: Language) {
  const dictionary = dictionaries[language as keyof typeof dictionaries] as Partial<Record<TranslationKey, string>> | undefined;
  return (key: TranslationKey) => dictionary?.[key] ?? dictionaries.en[key];
}
