import { create } from "zustand";
import { STRINGS, SUPPORTED_LOCALES, type Dict, type Locale } from "../i18n/strings";

const STORAGE_KEY = "rift.locale";

function detectInitial(): Locale {
  if (typeof window !== "undefined") {
    try {
      const stored = window.localStorage.getItem(STORAGE_KEY);
      if (stored && (SUPPORTED_LOCALES as string[]).includes(stored)) {
        return stored as Locale;
      }
    } catch {}
    const lang = (window.navigator?.language ?? "en").toLowerCase();
    if (lang.startsWith("ru")) return "ru";
  }
  return "en";
}

interface I18nState {
  locale: Locale;
  t: Dict;
  setLocale: (locale: Locale) => void;
}

export const useI18nStore = create<I18nState>()((set) => ({
  locale: detectInitial(),
  t: STRINGS[detectInitial()],
  setLocale: (locale) => {
    try {
      window.localStorage.setItem(STORAGE_KEY, locale);
    } catch {}
    set({ locale, t: STRINGS[locale] });
  },
}));
