import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";

import enAuth from "./locales/en/auth.json";
import enCall from "./locales/en/call.json";
import enChat from "./locales/en/chat.json";
import enCommon from "./locales/en/common.json";
import enMembers from "./locales/en/members.json";
import enModals from "./locales/en/modals.json";
import enServerSettings from "./locales/en/serverSettings.json";
import enSettings from "./locales/en/settings.json";
import enSidebar from "./locales/en/sidebar.json";
import enVoice from "./locales/en/voice.json";
import ruAuth from "./locales/ru/auth.json";
import ruCall from "./locales/ru/call.json";
import ruChat from "./locales/ru/chat.json";
import ruCommon from "./locales/ru/common.json";
import ruMembers from "./locales/ru/members.json";
import ruModals from "./locales/ru/modals.json";
import ruServerSettings from "./locales/ru/serverSettings.json";
import ruSettings from "./locales/ru/settings.json";
import ruSidebar from "./locales/ru/sidebar.json";
import ruVoice from "./locales/ru/voice.json";
import enHome from "./locales/en/home.json";
import ruHome from "./locales/ru/home.json";
import enProfile from "./locales/en/profile.json";
import ruProfile from "./locales/ru/profile.json";
import enSubscription from "./locales/en/subscription.json";
import ruSubscription from "./locales/ru/subscription.json";

export const SUPPORTED_LOCALES = ["en", "ru"] as const;
export type Locale = (typeof SUPPORTED_LOCALES)[number];

export const NAMESPACES = [
  "common",
  "auth",
  "sidebar",
  "settings",
  "chat",
  "voice",
  "modals",
  "members",
  "call",
  "serverSettings",
  "home",
  "profile",
  "subscription",
] as const;

const resources = {
  en: {
    common: enCommon,
    auth: enAuth,
    sidebar: enSidebar,
    settings: enSettings,
    chat: enChat,
    voice: enVoice,
    modals: enModals,
    members: enMembers,
    call: enCall,
    serverSettings: enServerSettings,
    home: enHome,
    profile: enProfile,
    subscription: enSubscription,
  },
  ru: {
    common: ruCommon,
    auth: ruAuth,
    sidebar: ruSidebar,
    settings: ruSettings,
    chat: ruChat,
    voice: ruVoice,
    modals: ruModals,
    members: ruMembers,
    call: ruCall,
    serverSettings: ruServerSettings,
    home: ruHome,
    profile: ruProfile,
    subscription: ruSubscription,
  },
} as const;

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: "en",
    supportedLngs: SUPPORTED_LOCALES as unknown as string[],
    nonExplicitSupportedLngs: true,
    load: "languageOnly",
    ns: NAMESPACES as unknown as string[],
    defaultNS: "common",
    interpolation: { escapeValue: false },
    returnNull: false,
    detection: {
      order: ["localStorage", "navigator"],
      lookupLocalStorage: "rift.locale",
      caches: ["localStorage"],
    },
  });

export function normalizeLocale(input: string | undefined): Locale {
  const lang = (input ?? "en").toLowerCase();
  const prefix = lang.split(/[-_]/)[0];
  return (SUPPORTED_LOCALES as readonly string[]).includes(prefix)
    ? (prefix as Locale)
    : "en";
}

export function setLocale(locale: Locale): void {
  if (i18n.language !== locale) {
    void i18n.changeLanguage(locale);
  }
}

export function currentLocale(): Locale {
  return normalizeLocale(i18n.language);
}

export default i18n;
