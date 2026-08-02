import { deMessages } from "./i18n/locales/de";
import { enMessages } from "./i18n/locales/en";
import { esMessages } from "./i18n/locales/es";
import { jaMessages } from "./i18n/locales/ja";
import { koMessages } from "./i18n/locales/ko";
import { ruMessages } from "./i18n/locales/ru";
import { zhCNMessages } from "./i18n/locales/zh-CN";
import { zhTWMessages } from "./i18n/locales/zh-TW";

export type Language = "en" | "es" | "de" | "ru" | "zh-TW" | "zh-CN" | "ja" | "ko";

export type TranslationDictionary = {
  [Key in keyof typeof zhCNMessages]: string;
};

export const dictionaries = {
  en: enMessages,
  es: esMessages,
  de: deMessages,
  ru: ruMessages,
  "zh-TW": zhTWMessages,
  "zh-CN": zhCNMessages,
  ja: jaMessages,
  ko: koMessages,
} satisfies Record<Language, TranslationDictionary>;

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

export type TranslationKey = keyof TranslationDictionary;

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
  const dictionary = dictionaries[language];
  return (key: TranslationKey) => dictionary[key];
}
