export const SETTINGS_SECTIONS = [
  "account",
  "privacy",
  "appearance",
  "voice",
  "notifications",
  "shortcuts",
  "language",
] as const;

export type SectionId = (typeof SETTINGS_SECTIONS)[number];
