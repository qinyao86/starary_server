import { en } from "./i18n/en";
import { zh } from "./i18n/zh";

export type Language = "en" | "zh";

export const dictionaries = { en, zh } as const;

export type TranslationKey = keyof typeof en;

export function createTranslator(language: Language) {
  return (key: TranslationKey) => dictionaries[language][key] ?? dictionaries.en[key];
}