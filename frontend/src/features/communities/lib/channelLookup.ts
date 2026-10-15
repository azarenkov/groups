import type { Channel } from "../../../shared/types/channel";
import { useUnreadStore } from "../../chat/store/unreadStore";
import { channelsFullyLoaded, useChannelStore } from "../store/channelStore";
import { useCommunityStore } from "../store/communityStore";

export function findLoadedChannel(channelId: string): { communityId: string; channel: Channel } | null {
  for (const [communityId, channels] of Object.entries(useChannelStore.getState().channelsByCommunity)) {
    const channel = channels.find((c) => c.id === channelId);
    if (channel) return { communityId, channel };
  }
  return null;
}

export async function resolveChannelCommunity(channelId: string): Promise<string | null> {
  const known = findLoadedChannel(channelId)?.communityId ?? useUnreadStore.getState().channelToCommunity[channelId];
  if (known) return known;
  for (const community of useCommunityStore.getState().communities) {
    if (channelsFullyLoaded(community.id)) continue;
    await useChannelStore.getState().fetchChannels(community.id);
    const found = findLoadedChannel(channelId);
    if (found) return found.communityId;
  }
  return null;
}
