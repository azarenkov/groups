const TOKEN_RE = /^[A-Za-z0-9_-]{22,64}$/;

export function extractInviteToken(input: string): string | null {
  const raw = input.trim();
  if (!raw) return null;
  const withoutQuery = raw.split(/[?#]/)[0];
  const segments = withoutQuery.split(/[/\s]+/).filter(Boolean);
  const candidate = (segments[segments.length - 1] ?? "").replace(/^\+/, "");
  return TOKEN_RE.test(candidate) ? candidate : null;
}

const PACK_SLUG_RE = /^[a-z0-9_]{3,32}$/;

export function extractPackSlug(input: string): string | null {
  const slug = input.trim().toLowerCase();
  return PACK_SLUG_RE.test(slug) ? slug : null;
}

export function authRedirectTarget(search: string): string {
  const next = new URLSearchParams(search).get("next");
  if (!next) return "/app";
  const pack = /^\/stickers\/([^/?#]+)$/.exec(next);
  if (pack) {
    const slug = extractPackSlug(pack[1]);
    return slug ? `/stickers/${slug}` : "/app";
  }
  const match = /^\/join\/([^/?#]+)$/.exec(next);
  if (!match) return "/app";
  const token = extractInviteToken(match[1]);
  return token ? `/join/${token}` : "/app";
}

export function appDeepLink(token: string): string {
  return `rift://join/${token}`;
}
