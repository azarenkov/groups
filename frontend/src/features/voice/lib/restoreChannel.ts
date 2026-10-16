import type { Channel } from "../../../shared/types/channel";
import type { Community } from "../../../shared/types/community";

export type ChannelRestore =
  | { status: "wait" }
  | { status: "drop" }
  | { status: "join"; community: Community; channel: Channel };

export function resolveChannelRestore(
  target: { communityId: string; channelId: string },
  communities: { list: Community[]; loaded: boolean },
  channels: { list: Channel[] | undefined; loaded: boolean },
): ChannelRestore {
  const community = communities.list.find((c) => c.id === target.communityId);
  if (!community) return communities.loaded ? { status: "drop" } : { status: "wait" };
  const channel = channels.list?.find((c) => c.id === target.channelId);
  if (!channel) return channels.loaded ? { status: "drop" } : { status: "wait" };
  if (channel.kind !== "voice" || channel.visibility === "private") return { status: "drop" };
  return { status: "join", community, channel };
}
