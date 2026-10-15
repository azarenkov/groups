const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

export function extractUuid(input: string): string | null {
  const id = input.trim().toLowerCase();
  return UUID_RE.test(id) ? id : null;
}
