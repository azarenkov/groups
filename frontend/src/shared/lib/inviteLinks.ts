import { isAppPath } from "../../features/navigation/lib/appLocation";
import { extractUuid } from "./ids";

const TOKEN_RE = /^[A-Za-z0-9_-]{22,64}$/;

const linkPath = (raw: string): string => raw.split(/[?#]/)[0];

export function extractInviteToken(input: string): string | null {
  const raw = input.trim();
  if (!raw || linkedProfileId(raw)) return null;
  const segments = linkPath(raw).split(/[/\s]+/).filter(Boolean);
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
  const profile = /^\/u\/([^/?#]+)$/.exec(next);
  if (profile) {
    const id = extractProfileId(profile[1]);
    return id ? `/u/${id}` : "/app";
  }
  if (isAppPath(next)) return next;
  const match = /^\/join\/([^/?#]+)$/.exec(next);
  if (!match) return "/app";
  const token = extractInviteToken(match[1]);
  return token ? `/join/${token}` : "/app";
}

export function appDeepLink(token: string): string {
  return `rift://join/${token}`;
}

const RESET_TOKEN_RE = /^[0-9a-f]{32}\.[0-9a-f]{64}$/;

export function resetPasswordDeepLink(token: string): string {
  return `rift://reset-password?token=${encodeURIComponent(token)}`;
}

export function extractResetToken(url: string): string | null {
  const match = /^rift:\/\/reset-password\/?\?token=([^&#]+)/.exec(url.trim());
  if (!match) return null;
  const token = decodeURIComponent(match[1]);
  return RESET_TOKEN_RE.test(token) ? token : null;
}

export const extractProfileId = extractUuid;

const PROFILE_PATH_RE = /(?:^|\/)u\/([^/\s]+)\/?$/;

function linkedProfileId(raw: string): string | null {
  const path = PROFILE_PATH_RE.exec(linkPath(raw));
  return path ? extractProfileId(path[1]) : null;
}

export function extractProfileLinkId(input: string): string | null {
  const raw = input.trim();
  return linkedProfileId(raw) ?? extractProfileId(raw);
}

export function profileDeepLink(userId: string): string {
  return `rift://u/${userId}`;
}

export function extractProfileDeepLink(url: string): string | null {
  const match = /^rift:\/\/u\/([^/?#]+)\/?$/.exec(url.trim());
  return match ? extractProfileId(match[1]) : null;
}
