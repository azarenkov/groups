import React, { useEffect, useState } from "react";
import { userAPI } from "../../social/api/user";
import { useCallStore } from "../../voice/store/callStore";
import { useUnreadStore } from "../../chat/store/unreadStore";
import type { useVoiceSession } from "../../voice/hooks/useVoiceSession";
import type { Channel } from "../../../shared/types/channel";
import type { Server } from "../../../shared/types/server";
import type { DirectConversation } from "../../dm/types";

export function useSelection(params: {
  hasSession: boolean;
  servers: Server[];
  channelsByServer: Record<string, Channel[]>;
  selectedServerId: string | null;
  selectedChannelId: string | null;
  setSelectedServer: (serverId: string | null) => void;
  setSelectedChannel: (channelId: string | null) => void;
  fetchServers: () => Promise<unknown>;
  fetchChannels: (serverId: string) => unknown;
  voice: ReturnType<typeof useVoiceSession>;
}) {
  const {
    hasSession,
    servers,
    channelsByServer,
    selectedServerId,
    selectedChannelId,
    setSelectedServer,
    setSelectedChannel,
    fetchServers,
    fetchChannels,
    voice,
  } = params;
  const { joinedChannel, voiceJoinFailed, setPasswordPrompt, startVoiceJoin } =
    voice;

  const [selectedDm, setSelectedDm] = useState<DirectConversation | null>(null);
  const [expandedServerIds, setExpandedServerIds] = useState<Set<string>>(
    () => new Set<string>(),
  );

  useEffect(() => {
    if (!hasSession) return;
    const existing = new Set(servers.map((s) => s.id));
    expandedServerIds.forEach((id) => {
      if (!existing.has(id)) return;
      if (!channelsByServer[id]) fetchChannels(id);
    });
  }, [expandedServerIds, hasSession, fetchChannels, channelsByServer, servers]);

  const didInitialExpandRef = React.useRef(false);
  useEffect(() => {
    if (didInitialExpandRef.current) return;
    if (servers.length === 0) return;
    didInitialExpandRef.current = true;
    setExpandedServerIds(new Set([servers[0].id]));
  }, [servers]);

  useEffect(() => {
    if (!selectedServerId || selectedDm) return;
    const list = channelsByServer[selectedServerId];
    if (!list) return;
    if (selectedChannelId) {
      const current = list.find((c) => c.id === selectedChannelId);
      if (current) return;
    }
    const firstText = list.find((c) => c.kind === "text");
    if (firstText) setSelectedChannel(firstText.id);
  }, [
    selectedServerId,
    selectedChannelId,
    selectedDm,
    channelsByServer,
    setSelectedChannel,
  ]);

  const selectedServer = servers.find((s) => s.id === selectedServerId) || null;

  const selectedTextChannel =
    selectedServer && selectedChannelId
      ? channelsByServer[selectedServer.id]?.find(
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
    selectedServer && selectedChannelId
      ? channelsByServer[selectedServer.id]?.find(
          (c) =>
            c.id === selectedChannelId &&
            c.kind === "voice" &&
            joinedChannel?.channel.id !== c.id,
        ) ?? null
      : null;

  const selectedServerChannels = selectedServer
    ? channelsByServer[selectedServer.id]
    : undefined;
  const selectedServerChannelsLoaded = selectedServerChannels !== undefined;
  const serverHasTextChannel =
    selectedServerChannelsLoaded &&
    selectedServerChannels!.some((c) => c.kind === "text");

  const viewingJoinedVoice =
    !!joinedChannel && selectedChannelId === joinedChannel.channel.id;
  const onTextChannelActive =
    !!selectedServer && !selectedDm && !viewingJoinedVoice;

  const expandServer = (serverId: string) =>
    setExpandedServerIds((prev) => {
      if (prev.has(serverId)) return prev;
      const next = new Set(prev);
      next.add(serverId);
      return next;
    });

  const collapseServer = (serverId: string) =>
    setExpandedServerIds((prev) => {
      if (!prev.has(serverId)) return prev;
      const next = new Set(prev);
      next.delete(serverId);
      return next;
    });

  const handleToggleServer = (serverId: string) => {
    setExpandedServerIds((prev) => {
      const next = new Set(prev);
      if (next.has(serverId)) next.delete(serverId);
      else next.add(serverId);
      return next;
    });
  };

  const handleSelectServer = (serverId: string) => {
    useCallStore.getState().setFocused(false);
    setSelectedServer(serverId);
    setSelectedDm(null);
    const firstText = channelsByServer[serverId]?.find((c) => c.kind === "text");
    setSelectedChannel(firstText ? firstText.id : null);
  };

  const finalizeJoin = (server: Server, channel: Channel, password?: string) => {
    setSelectedServer(server.id);
    setSelectedChannel(channel.id);
    setSelectedDm(null);
    startVoiceJoin(server, channel, password);
  };

  const handleSelectChannel = (server: Server, channel: Channel) => {
    useCallStore.getState().setFocused(false);
    if (channel.kind === "text") {
      setSelectedServer(server.id);
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
      setSelectedServer(server.id);
      setSelectedChannel(channel.id);
      setSelectedDm(null);
      return;
    }
    if (channel.visibility === "private") {
      setPasswordPrompt({ server, channel });
      return;
    }
    finalizeJoin(server, channel);
  };

  const handleSelectDm = (dm: DirectConversation) => {
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

  const handleGoToServer = async (serverId: string) => {
    await fetchServers();
    expandServer(serverId);
    setSelectedServer(serverId);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  const handleGoHome = () => {
    useCallStore.getState().setFocused(false);
    setSelectedServer(null);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  const openServer = (serverId: string) => {
    expandServer(serverId);
    setSelectedServer(serverId);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  return {
    selectedDm,
    setSelectedDm,
    expandedServerIds,
    expandServer,
    collapseServer,
    selectedServer,
    activeTextChannel,
    selectedVoiceChannelNotJoined,
    selectedServerChannelsLoaded,
    serverHasTextChannel,
    onTextChannelActive,
    handleToggleServer,
    handleSelectServer,
    handleSelectChannel,
    handleSelectDm,
    handleGoToServer,
    handleGoHome,
    finalizeJoin,
    openServer,
  };
}
