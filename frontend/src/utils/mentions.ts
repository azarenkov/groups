import { URL_REGEX } from "./links";

export const MENTION_REGEX = /(^|[^A-Za-z0-9_@])@([A-Za-z0-9_]+)/g;

function stripUrls(content: string): string {
  return content.replace(URL_REGEX, " ");
}

export function mentionsUser(content: string, name: string): boolean {
  if (!name) return false;
  const lower = name.toLowerCase();
  for (const match of stripUrls(content).matchAll(MENTION_REGEX)) {
    if (match[2].toLowerCase() === lower) return true;
  }
  return false;
}

export function activeMentionQuery(
  text: string,
  caret: number,
): { query: string; start: number } | null {
  const before = text.slice(0, caret);
  const at = before.lastIndexOf("@");
  if (at === -1) return null;
  if (at > 0 && /[A-Za-z0-9_@]/.test(before[at - 1])) return null;
  const query = before.slice(at + 1);
  if (!/^[A-Za-z0-9_]*$/.test(query)) return null;
  for (const match of text.matchAll(URL_REGEX)) {
    const start = match.index ?? 0;
    if (at >= start && at < start + match[0].length) return null;
  }
  return { query, start: at };
}
