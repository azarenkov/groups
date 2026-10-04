export type MessageLinkTarget =
  | { kind: "channel"; communityId: string; channelId: string; messageId: string }
  | { kind: "dm"; participants: [string, string]; messageId: string };

export function messageLink(origin: string, target: MessageLinkTarget): string {
  const url = new URL("/app", origin);
  if (target.kind === "channel") {
    url.searchParams.set("community", target.communityId);
    url.searchParams.set("channel", target.channelId);
  } else {
    url.searchParams.set("dm", [...target.participants].sort().join("~"));
  }
  url.searchParams.set("message", target.messageId);
  return url.toString();
}

export type OpenMessageLink =
  | { kind: "channel"; communityId: string; channelId: string; messageId: string }
  | { kind: "dm"; peerId: string; messageId: string; peerName?: string };

export function parseMessageLink(
  params: URLSearchParams,
  viewerId: string | undefined,
): OpenMessageLink | null {
  const messageId = params.get("message");
  if (!messageId) return null;
  const community = params.get("community");
  const channel = params.get("channel");
  if (community && channel) {
    return { kind: "channel", communityId: community, channelId: channel, messageId };
  }
  const pair = params.get("dm")?.split("~");
  if (!pair || pair.length !== 2 || !viewerId || !pair.includes(viewerId)) return null;
  const peerId = pair[0] === viewerId ? pair[1] : pair[0];
  if (!peerId || peerId === viewerId) return null;
  return { kind: "dm", peerId, messageId };
}
