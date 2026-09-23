export const MAX_REPORT_DETAILS = 1000;

export const countReportDetails = (value: string) => [...value].length;

export const detailsLengthAfterEdit = (
  current: string,
  selectionStart: number,
  selectionEnd: number,
  inserted: string,
) =>
  countReportDetails(current) -
  countReportDetails(current.slice(selectionStart, selectionEnd)) +
  countReportDetails(inserted);

export const USER_REPORT_REASONS = [
  "spam",
  "harassment",
  "hate",
  "threats",
  "scam",
  "nsfw",
  "csam",
  "self_harm",
  "impersonation",
  "illegal",
  "other",
] as const;

export const COMMUNITY_REPORT_REASONS = [
  "spam",
  "harassment",
  "hate",
  "violence",
  "scam",
  "nsfw",
  "csam",
  "illegal",
  "raid",
  "other",
] as const;

export type ReportReason =
  | (typeof USER_REPORT_REASONS)[number]
  | (typeof COMMUNITY_REPORT_REASONS)[number];
