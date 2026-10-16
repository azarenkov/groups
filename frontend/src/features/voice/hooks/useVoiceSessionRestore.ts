import { useEffect, useRef } from "react";

import { channelsFullyLoaded } from "../../communities/store/channelStore";
import type { Channel } from "../../../shared/types/channel";
import type { Community } from "../../../shared/types/community";
import { resolveChannelRestore } from "../lib/restoreChannel";
import { forgetVoiceSession, freshVoiceSession } from "../lib/sessionHint";

export function useVoiceSessionRestore(params: {
  userId: string | undefined;
  communities: Community[];
  communitiesLoaded: boolean;
  channelsByCommunity: Record<string, Channel[]>;
  fetchChannels: (communityId: string) => unknown;
  rejoin: (community: Community, channel: Channel) => void;
}): void {
  const { userId, communities, communitiesLoaded, channelsByCommunity, fetchChannels, rejoin } = params;
  const settled = useRef(false);
  const requested = useRef<string | null>(null);
  const rejoinRef = useRef(rejoin);
  rejoinRef.current = rejoin;

  useEffect(() => {
    if (settled.current || !userId) return;
    const target = freshVoiceSession(userId);
    if (target?.kind !== "channel") {
      settled.current = true;
      return;
    }
    const loaded = channelsFullyLoaded(target.communityId);
    const outcome = resolveChannelRestore(
      target,
      { list: communities, loaded: communitiesLoaded },
      { list: channelsByCommunity[target.communityId], loaded },
    );
    if (outcome.status === "wait") {
      const known = communities.some((c) => c.id === target.communityId);
      if (known && !loaded && requested.current !== target.communityId) {
        requested.current = target.communityId;
        fetchChannels(target.communityId);
      }
      return;
    }
    settled.current = true;
    if (outcome.status === "drop") {
      forgetVoiceSession(target);
      return;
    }
    rejoinRef.current(outcome.community, outcome.channel);
  }, [userId, communities, communitiesLoaded, channelsByCommunity, fetchChannels]);
}
