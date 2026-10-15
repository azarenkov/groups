export const SETTINGS_SECTIONS = [
  "account",
  "privacy",
  "connections",
  "subscription",
  "appearance",
  "voice",
  "notifications",
  "shortcuts",
  "language",
] as const;

export type SectionId = (typeof SETTINGS_SECTIONS)[number];
