import { extractUuid } from "../../../shared/lib/ids";
import { SETTINGS_SECTIONS, type SectionId } from "../../settings/components/types";

export type ChatLocation =
  | { kind: "dm"; peerId: string; messageId?: string; peerName?: string }
  | { kind: "channel"; communityId: string; channelId: string; messageId?: string };

export type AppLocation = { kind: "home" } | { kind: "settings"; section: SectionId } | ChatLocation;

export type MessageLinkTarget =
  | { kind: "channel"; communityId: string; channelId: string; messageId: string }
  | { kind: "dm"; participants: [string, string]; messageId: string };

export const APP_ROOT = "/app";

const DM_PAIR_SEPARATOR = "~";

const isSection = (value: string): value is SectionId => (SETTINGS_SECTIONS as readonly string[]).includes(value);

type ParsedPath =
  | { kind: "home" }
  | { kind: "settings"; section: SectionId }
  | { kind: "dm"; participants: string[]; messageId?: string }
  | { kind: "channel"; communityId: string; channelId: string; messageId?: string };

function participantsOf(segment: string): string[] | null {
  const ids = segment.split(DM_PAIR_SEPARATOR).map(extractUuid);
  if (ids.length > 2 || ids.some((id) => !id) || (ids.length === 2 && ids[0] === ids[1])) return null;
  return ids as string[];
}

function peerOf(participants: string[], viewerId: string | undefined): string | null {
  if (participants.length === 1) return participants[0] === viewerId ? null : participants[0];
  if (!viewerId || !participants.includes(viewerId)) return null;
  return participants.find((id) => id !== viewerId) ?? null;
}

function parsePath(pathname: string): ParsedPath | null {
  const segments = pathname.split("/").filter(Boolean);
  if (segments[0] !== APP_ROOT.slice(1)) return null;
  const [, kind, ...rest] = segments;
  if (kind === "home" && rest.length === 0) return { kind: "home" };
  if (kind === "settings" && rest.length === 1 && isSection(rest[0])) return { kind: "settings", section: rest[0] };
  const messageId = rest.length === (kind === "dm" ? 2 : 3) ? extractUuid(rest[rest.length - 1]) : undefined;
  if (messageId === null) return null;
  if (kind === "dm" && (rest.length === 1 || rest.length === 2)) {
    const participants = participantsOf(rest[0]);
    return participants ? { kind: "dm", participants, ...(messageId ? { messageId } : {}) } : null;
  }
  if (kind === "c" && (rest.length === 2 || rest.length === 3)) {
    const communityId = extractUuid(rest[0]);
    const channelId = extractUuid(rest[1]);
    if (!communityId || !channelId) return null;
    return { kind: "channel", communityId, channelId, ...(messageId ? { messageId } : {}) };
  }
  return null;
}

export const isAppPath = (pathname: string): boolean => parsePath(pathname) !== null;

export function parseAppPath(pathname: string, viewerId: string | undefined): AppLocation | null {
  const parsed = parsePath(pathname);
  if (!parsed || parsed.kind !== "dm") return parsed;
  const peerId = peerOf(parsed.participants, viewerId);
  if (!peerId) return null;
  return parsed.messageId ? { kind: "dm", peerId, messageId: parsed.messageId } : { kind: "dm", peerId };
}

export function appPath(location: AppLocation): string {
  switch (location.kind) {
    case "home":
      return `${APP_ROOT}/home`;
    case "settings":
      return `${APP_ROOT}/settings/${location.section}`;
    case "dm":
      return [APP_ROOT, "dm", location.peerId, location.messageId].filter(Boolean).join("/");
    case "channel":
      return [APP_ROOT, "c", location.communityId, location.channelId, location.messageId].filter(Boolean).join("/");
  }
}

const DEEP_LINK_PREFIX = "rift://app/";

export function appPathFromDeepLink(url: string): string | null {
  const raw = url.trim();
  if (!raw.startsWith(DEEP_LINK_PREFIX)) return null;
  const path = `${APP_ROOT}/${raw.slice(DEEP_LINK_PREFIX.length)}`.replace(/\/+$/, "");
  return isAppPath(path) ? path : null;
}

export function messageLinkPath(target: MessageLinkTarget): string {
  if (target.kind === "channel") return appPath(target);
  return [APP_ROOT, "dm", [...target.participants].sort().join(DM_PAIR_SEPARATOR), target.messageId].join("/");
}

export function sameLocation(a: AppLocation | null, b: AppLocation | null): boolean {
  if (!a || !b || a.kind !== b.kind) return a === b;
  switch (a.kind) {
    case "home":
      return true;
    case "settings":
      return a.section === (b as typeof a).section;
    case "dm":
      return a.peerId === (b as typeof a).peerId;
    case "channel":
      return a.communityId === (b as typeof a).communityId && a.channelId === (b as typeof a).channelId;
  }
}

export function legacyMessageLink(params: URLSearchParams, viewerId: string | undefined): ChatLocation | null {
  const messageId = params.get("message");
  if (!messageId) return null;
  const community = params.get("community");
  const channel = params.get("channel");
  if (community && channel) return { kind: "channel", communityId: community, channelId: channel, messageId };
  const participants = participantsOf(params.get("dm") ?? "");
  const peerId = participants && participants.length === 2 ? peerOf(participants, viewerId) : null;
  return peerId ? { kind: "dm", peerId, messageId } : null;
}
