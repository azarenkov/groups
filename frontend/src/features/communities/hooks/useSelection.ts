import React, { useEffect, useState } from "react";
import { channelsFullyLoaded } from "../store/channelStore";
import { userAPI } from "../../social/api/user";
import { useCallStore } from "../../voice/store/callStore";
import { useUnreadStore } from "../../chat/store/unreadStore";
import type { useVoiceSession } from "../../voice/hooks/useVoiceSession";
import type { Channel } from "../../../shared/types/channel";
import type { Community } from "../../../shared/types/community";
import type { DirectConversation } from "../../dm/types";

export function useSelection(params: {
  hasSession: boolean;
  communities: Community[];
  channelsByCommunity: Record<string, Channel[]>;
  selectedCommunityId: string | null;
  selectedChannelId: string | null;
  setSelectedCommunity: (communityId: string | null) => void;
  setSelectedChannel: (channelId: string | null) => void;
  fetchCommunities: () => Promise<unknown>;
  fetchChannels: (communityId: string) => unknown;
  voice: ReturnType<typeof useVoiceSession>;
}) {
  const {
    hasSession,
    communities,
    channelsByCommunity,
    selectedCommunityId,
    selectedChannelId,
    setSelectedCommunity,
    setSelectedChannel,
    fetchCommunities,
    fetchChannels,
    voice,
  } = params;
  const { joinedChannel, voiceJoinFailed, setPasswordPrompt, startVoiceJoin } =
    voice;

  const [selectedDm, setSelectedDm] = useState<DirectConversation | null>(null);
  const [selectionSeq, setSelectionSeq] = useState(0);
  const noteSelection = () => setSelectionSeq((n) => n + 1);
  const [expandedCommunityIds, setExpandedCommunityIds] = useState<Set<string>>(
    () => new Set<string>(),
  );

  useEffect(() => {
    if (!hasSession) return;
    const existing = new Set(communities.map((s) => s.id));
    expandedCommunityIds.forEach((id) => {
      if (!existing.has(id)) return;
      if (!channelsFullyLoaded(id)) fetchChannels(id);
    });
  }, [expandedCommunityIds, hasSession, fetchChannels, channelsByCommunity, communities]);

  const didInitialExpandRef = React.useRef(false);
  useEffect(() => {
    if (didInitialExpandRef.current) return;
    if (communities.length === 0) return;
    didInitialExpandRef.current = true;
    setExpandedCommunityIds(new Set([communities[0].id]));
  }, [communities]);

  useEffect(() => {
    if (!selectedCommunityId || selectedDm) return;
    const list = channelsByCommunity[selectedCommunityId];
    if (!list) return;
    if (selectedChannelId) {
      const current = list.find((c) => c.id === selectedChannelId);
      if (current || !channelsFullyLoaded(selectedCommunityId)) return;
    }
    const firstText = list.find((c) => c.kind === "text");
    if (firstText) setSelectedChannel(firstText.id);
  }, [
    selectedCommunityId,
    selectedChannelId,
    selectedDm,
    channelsByCommunity,
    setSelectedChannel,
  ]);

  const selectedCommunity = communities.find((s) => s.id === selectedCommunityId) || null;

  const selectedTextChannel =
    selectedCommunity && selectedChannelId
      ? channelsByCommunity[selectedCommunity.id]?.find(
          (c) => c.id === selectedChannelId && c.kind === "text",
        ) ?? null
      : null;

  const activeTextChannel = selectedTextChannel
    ? {
        id: selectedTextChannel.id,
        name: selectedTextChannel.name,
        topic: undefined as string | undefined,
      }
    : null;

  const selectedVoiceChannelNotJoined =
    selectedCommunity && selectedChannelId
      ? channelsByCommunity[selectedCommunity.id]?.find(
          (c) =>
            c.id === selectedChannelId &&
            c.kind === "voice" &&
            joinedChannel?.channel.id !== c.id,
        ) ?? null
      : null;

  const selectedCommunityChannels = selectedCommunity
    ? channelsByCommunity[selectedCommunity.id]
    : undefined;
  const selectedCommunityChannelsLoaded = selectedCommunityChannels !== undefined;
  const communityHasTextChannel =
    selectedCommunityChannelsLoaded &&
    selectedCommunityChannels!.some((c) => c.kind === "text");

  const viewingJoinedVoice =
    !!joinedChannel && selectedChannelId === joinedChannel.channel.id;
  const onTextChannelActive =
    !!selectedCommunity && !selectedDm && !viewingJoinedVoice;

  const expandCommunity = (communityId: string) =>
    setExpandedCommunityIds((prev) => {
      if (prev.has(communityId)) return prev;
      const next = new Set(prev);
      next.add(communityId);
      return next;
    });

  const collapseCommunity = (communityId: string) =>
    setExpandedCommunityIds((prev) => {
      if (!prev.has(communityId)) return prev;
      const next = new Set(prev);
      next.delete(communityId);
      return next;
    });

  const handleToggleCommunity = (communityId: string) => {
    setExpandedCommunityIds((prev) => {
      const next = new Set(prev);
      if (next.has(communityId)) next.delete(communityId);
      else next.add(communityId);
      return next;
    });
  };

  const handleSelectCommunity = (communityId: string) => {
    noteSelection();
    useCallStore.getState().setFocused(false);
    setSelectedCommunity(communityId);
    setSelectedDm(null);
    const firstText = channelsByCommunity[communityId]?.find((c) => c.kind === "text");
    setSelectedChannel(firstText ? firstText.id : null);
  };

  const finalizeJoin = (community: Community, channel: Channel, password?: string) => {
    setSelectedCommunity(community.id);
    setSelectedChannel(channel.id);
    setSelectedDm(null);
    startVoiceJoin(community, channel, password);
  };

  const handleSelectChannel = (community: Community, channel: Channel) => {
    noteSelection();
    useCallStore.getState().setFocused(false);
    if (channel.kind === "text") {
      setSelectedCommunity(community.id);
      setSelectedChannel(channel.id);
      setSelectedDm(null);
      useUnreadStore.getState().resetChannel(channel.id);
      userAPI.markChannelRead(channel.id).catch((err) => {
        console.warn("[read-state] mark channel read failed:", err);
      });
      return;
    }
    if (
      joinedChannel &&
      joinedChannel.channel.id === channel.id &&
      !voiceJoinFailed
    ) {
      setSelectedCommunity(community.id);
      setSelectedChannel(channel.id);
      setSelectedDm(null);
      return;
    }
    if (channel.visibility === "private") {
      setPasswordPrompt({ community, channel });
      return;
    }
    finalizeJoin(community, channel);
  };

  const handleSelectDm = (dm: DirectConversation) => {
    noteSelection();
    const cs = useCallStore.getState();
    const inCallWithDm =
      !!cs.activeCallId && (cs.peerId === dm.id || dm.id in cs.participants);
    cs.setFocused(inCallWithDm);
    setSelectedDm(dm);
    setSelectedChannel(null);
    useUnreadStore.getState().resetDm(dm.id);
    userAPI.markDmRead(dm.id).catch((err) => {
      console.warn("[read-state] mark dm read failed:", err);
    });
  };

  const handleGoToCommunity = async (communityId: string) => {
    noteSelection();
    await fetchCommunities();
    expandCommunity(communityId);
    setSelectedCommunity(communityId);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  const handleGoHome = () => {
    noteSelection();
    useCallStore.getState().setFocused(false);
    setSelectedCommunity(null);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  const openCommunity = (communityId: string) => {
    noteSelection();
    expandCommunity(communityId);
    setSelectedCommunity(communityId);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  return {
    selectedDm,
    setSelectedDm,
    selectionSeq,
    expandedCommunityIds,
    expandCommunity,
    collapseCommunity,
    selectedCommunity,
    activeTextChannel,
    selectedVoiceChannelNotJoined,
    selectedCommunityChannelsLoaded,
    communityHasTextChannel,
    onTextChannelActive,
    handleToggleCommunity,
    handleSelectCommunity,
    handleSelectChannel,
    handleSelectDm,
    handleGoToCommunity,
    handleGoHome,
    finalizeJoin,
    openCommunity,
  };
}
