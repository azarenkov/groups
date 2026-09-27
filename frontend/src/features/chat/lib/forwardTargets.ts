import type { Channel } from "../../../shared/types/channel";
import type { Community } from "../../../shared/types/community";
import type { Friendship } from "../../../shared/types/friendship";
import type { ForwardTarget } from "../api/forward";

export const MAX_FORWARD_TARGETS = 10;

export interface ForwardOption {
  key: string;
  label: string;
  sublabel?: string;
  avatarId: string;
  target: ForwardTarget;
}

export function forwardTargetKey(target: ForwardTarget): string {
  return target.kind === "dm"
    ? `dm:${target.friendId}`
    : `channel:${target.communityId}:${target.channelId}`;
}

export function buildForwardOptions(
  friends: Friendship[],
  communities: Community[],
  channelsByCommunity: Record<string, Channel[]>,
  query: string,
): { dms: ForwardOption[]; channels: ForwardOption[] } {
  const needle = query.trim().toLowerCase();
  const matches = (...parts: (string | undefined)[]) =>
    needle.length === 0 || parts.some((p) => p?.toLowerCase().includes(needle));
  const dms = friends
    .filter((f) => f.status === "accepted" && matches(f.friendName))
    .map((f) => {
      const target: ForwardTarget = { kind: "dm", friendId: f.friendId };
      return { key: forwardTargetKey(target), label: f.friendName, avatarId: f.friendId, target };
    });
  const channels: ForwardOption[] = [];
  for (const community of communities) {
    for (const channel of channelsByCommunity[community.id] ?? []) {
      if (channel.kind !== "text" || !matches(channel.name, community.name)) continue;
      const target: ForwardTarget = {
        kind: "channel",
        communityId: community.id,
        channelId: channel.id,
      };
      channels.push({
        key: forwardTargetKey(target),
        label: `#${channel.name}`,
        sublabel: community.name,
        avatarId: community.id,
        target,
      });
    }
  }
  return { dms, channels };
}

export function toggleForwardTarget(
  selected: ForwardTarget[],
  target: ForwardTarget,
): ForwardTarget[] {
  const key = forwardTargetKey(target);
  if (selected.some((t) => forwardTargetKey(t) === key)) {
    return selected.filter((t) => forwardTargetKey(t) !== key);
  }
  if (selected.length >= MAX_FORWARD_TARGETS) return selected;
  return [...selected, target];
}
