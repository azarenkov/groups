export const MENTION_REGEX = /(^|[^A-Za-z0-9_@])@([A-Za-z0-9_]+)/g;

export function mentionsUser(content: string, name: string): boolean {
  if (!name) return false;
  const lower = name.toLowerCase();
  for (const match of content.matchAll(MENTION_REGEX)) {
    if (match[2].toLowerCase() === lower) return true;
  }
  return false;
}

export function extractMentionNames(content: string): string[] {
  const names: string[] = [];
  for (const match of content.matchAll(MENTION_REGEX)) {
    const name = match[2].toLowerCase();
    if (!names.includes(name)) names.push(name);
  }
  return names;
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
  return { query, start: at };
}
