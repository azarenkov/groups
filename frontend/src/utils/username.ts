export type UsernameIssue =
  | "required"
  | "tooShort"
  | "tooLong"
  | "charset"
  | "underscoreEdge"
  | "doubleUnderscore"
  | "digitsOnly"
  | "reserved";

const RESERVED = new Set([
  "admin",
  "administrator",
  "moderator",
  "mod",
  "system",
  "rift",
  "support",
  "help",
  "root",
  "owner",
  "staff",
  "everyone",
  "here",
  "me",
  "null",
  "undefined",
  "deleted",
  "unknown",
]);

export function validateUsername(raw: string): UsernameIssue | null {
  const value = raw.trim();
  if (!value) return "required";
  if (value.length < 3) return "tooShort";
  if (value.length > 15) return "tooLong";
  if (!/^[a-zA-Z0-9_]+$/.test(value)) return "charset";
  if (value.startsWith("_") || value.endsWith("_")) return "underscoreEdge";
  if (value.includes("__")) return "doubleUnderscore";
  if (/^[0-9]+$/.test(value)) return "digitsOnly";
  if (RESERVED.has(value.toLowerCase())) return "reserved";
  return null;
}
