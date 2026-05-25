import React, { useCallback, useEffect, useRef, useState } from "react";
import { useAuthStore } from "../store/authStore";
import { useServerStore } from "../store/serverStore";
import { useChannelStore } from "../store/channelStore";
import { useThemeStore } from "../store/themeStore";
import { serverAPI } from "../api/server";
import { channelAPI } from "../api/channel";
import CreateServerModal from "../components/CreateServerModal";
import CreateChannelModal from "../components/CreateChannelModal";
import PasswordPromptModal from "../components/PasswordPromptModal";
import NewDmModal from "../components/NewDmModal";
import AddFriendModal from "../components/AddFriendModal";
import PendingFriendsModal from "../components/PendingFriendsModal";
import InviteFriendsModal from "../components/InviteFriendsModal";
import { useFriendshipStore } from "../store/friendshipStore";
import { useVoicePresenceStore } from "../store/voicePresenceStore";
import { getLiveKitService } from "../services/livekit";
import { ws } from "../services/websocket";
import { friendshipAPI } from "../api/friendship";
import { dmAPI } from "../api/dm";
import { colorForId, initialFor } from "../utils/avatar";
import ChannelSettingsModal from "../components/ChannelSettingsModal";
import ConfirmModal from "../components/rift/ConfirmModal";
import Sidebar from "../components/rift/Sidebar";
import ChatView from "../components/rift/ChatView";
import VoiceView from "../components/rift/VoiceView";
import MembersPanel from "../components/rift/MembersPanel";
import SettingsView from "../components/settings/SettingsView";
import Icon from "../components/rift/Icon";
import { useLocalQuality } from "../hooks/useLocalQuality";
import { useParticipants } from "../hooks/useParticipants";
import { qualityClass, qualityLabel } from "../utils/quality";
import {
  Channel,
  ChannelKind,
  ChannelParticipant,
  ChannelVisibility,
} from "../types/channel";
import { Server } from "../types/server";
import {
  type MockDirect,
  type MockMember,
} from "../data/mockData";
import "../styles/rift.css";

interface ChannelOverride {
  name?: string;
  topic?: string;
}

const ServersPage: React.FC = () => {
  const localQuality = useLocalQuality();
  const voiceParticipants = useParticipants();
  const voicePresenceByChannel = useVoicePresenceStore((s) => s.byChannel);
  const [tileVolumes, setTileVolumes] = useState<Record<string, number>>({});
  const [openVolumeFor, setOpenVolumeFor] = useState<string | null>(null);
  const volumePopoverRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!openVolumeFor) return;
    const onDoc = (e: MouseEvent) => {
      if (
        volumePopoverRef.current &&
        !volumePopoverRef.current.contains(e.target as Node)
      ) {
        setOpenVolumeFor(null);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpenVolumeFor(null);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [openVolumeFor]);
  const { tokens, user, signOut } = useAuthStore();
  const {
    servers,
    selectedServerId,
    isLoading: serversLoading,
    error: serversError,
    setSelectedServer,
    fetchServers,
    createServer,
    deleteServer,
    clearError: clearServersError,
  } = useServerStore();

  const {
    channelsByServer,
    selectedChannelId,
    error: channelsError,
    setSelectedChannel,
    fetchChannels,
    createChannel,
    deleteChannel,
    clearError: clearChannelsError,
    clearChannels,
  } = useChannelStore();

  const { theme, setTheme } = useThemeStore();
  const [accent, setAccent] = useState("#0071e3");

  useEffect(() => {
    document.documentElement.style.setProperty("--accent", accent);
    document.documentElement.style.setProperty(
      "--accent-hover",
      `color-mix(in srgb, ${accent} 88%, #000)`,
    );
  }, [accent]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        setSettingsOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const [isServerModalOpen, setIsServerModalOpen] = useState(false);
  const [isChannelModalOpen, setIsChannelModalOpen] = useState(false);
  const [channelModalServerId, setChannelModalServerId] = useState<string | null>(null);
  const [isDmModalOpen, setIsDmModalOpen] = useState(false);
  const [isAddFriendOpen, setIsAddFriendOpen] = useState(false);
  const [isPendingOpen, setIsPendingOpen] = useState(false);
  const [inviteModalServer, setInviteModalServer] = useState<Server | null>(null);
  const friends = useFriendshipStore((s) => s.friends);
  const fetchFriends = useFriendshipStore((s) => s.fetchFriends);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    type: "server" | "leave-server" | "channel" | "friend";
    id: string;
    serverId?: string;
    name: string;
  } | null>(null);

  const [joinedChannel, setJoinedChannel] = useState<{
    channel: Channel;
    serverId: string;
    password?: string;
  } | null>(null);
  const [voiceJoinFailed, setVoiceJoinFailed] = useState(false);
  const [voiceJoinAttempt, setVoiceJoinAttempt] = useState(0);
  const [passwordPrompt, setPasswordPrompt] = useState<{
    server: Server;
    channel: Channel;
  } | null>(null);
  const [selectedDm, setSelectedDm] = useState<MockDirect | null>(null);
  const [expandedServerIds, setExpandedServerIds] = useState<Set<string>>(
    () => new Set<string>(),
  );
  const [directs, setDirects] = useState<MockDirect[]>([]);
  const [sidebarQuery, setSidebarQuery] = useState("");

  const acceptedFriends: MockDirect[] = React.useMemo(
    () =>
      friends
        .filter((f) => f.status === "accepted")
        .map((f) => ({
          id: f.friendId,
          name: f.friendName,
          initials: initialFor(f.friendName),
          color: colorForId(f.friendId),
          online: false,
          unread: 0,
        })),
    [friends],
  );

  const pendingCount = React.useMemo(
    () =>
      friends.filter(
        (f) =>
          f.status === "pending_incoming" ||
          f.status === "pending_outgoing" ||
          f.status === "declined",
      ).length,
    [friends],
  );

  const [channelOverrides, setChannelOverrides] =
    useState<Record<string, ChannelOverride>>({});
  const [editingChannel, setEditingChannel] = useState<{
    id: string;
    serverId: string;
    name: string;
    topic?: string;
  } | null>(null);

  const [membersOpenById, setMembersOpenById] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (tokens?.accessToken) {
      serverAPI.setAuthToken(tokens.accessToken);
      channelAPI.setAuthToken(tokens.accessToken);
      friendshipAPI.setAuthToken(tokens.accessToken);
      dmAPI.setAuthToken(tokens.accessToken);
      fetchServers();
      fetchFriends();
    }
  }, [tokens, fetchServers, fetchFriends]);

  useEffect(() => {
    if (!tokens?.accessToken) return;
    const existing = new Set(servers.map((s) => s.id));
    expandedServerIds.forEach((id) => {
      if (!existing.has(id)) return;
      if (!channelsByServer[id]) fetchChannels(id);
    });
  }, [expandedServerIds, tokens, fetchChannels, channelsByServer, servers]);

  const presenceSyncedRef = useRef<Set<string>>(new Set());
  const [presenceSyncTick, setPresenceSyncTick] = useState(0);
  useEffect(() => {
    return ws.onOpen(() => {
      presenceSyncedRef.current.clear();
      setPresenceSyncTick((n) => n + 1);
    });
  }, []);
  useEffect(() => {
    if (!tokens?.accessToken) return;
    const synced = presenceSyncedRef.current;
    servers.forEach((s) => {
      if (synced.has(s.id)) return;
      synced.add(s.id);
      serverAPI
        .listVoicePresence(s.id)
        .then((items) => {
          const groups: Record<
            string,
            { id: string; name: string; isMuted: boolean }[]
          > = {};
          for (const item of items) {
            (groups[item.channelId] ||= []).push({
              id: item.userId,
              name: item.userName,
              isMuted: item.isMuted,
            });
          }
          for (const [channelId, members] of Object.entries(groups)) {
            useVoicePresenceStore.getState().setMembers(channelId, members);
          }
        })
        .catch((err) => {
          synced.delete(s.id);
          console.warn(
            "[voice-presence] initial sync failed for server",
            s.id,
            err,
          );
        });
    });
  }, [servers, tokens, presenceSyncTick]);

  const prefetchInflightRef = useRef<Set<string>>(new Set());
  useEffect(() => {
    if (!tokens?.accessToken) return;
    if (!sidebarQuery.trim()) return;
    const handle = window.setTimeout(() => {
      const inflight = prefetchInflightRef.current;
      servers.forEach((s) => {
        if (channelsByServer[s.id]) return;
        if (inflight.has(s.id)) return;
        inflight.add(s.id);
        Promise.resolve(fetchChannels(s.id)).finally(() => {
          inflight.delete(s.id);
        });
      });
    }, 250);
    return () => window.clearTimeout(handle);
  }, [sidebarQuery, tokens, servers, channelsByServer, fetchChannels]);

  const didInitialExpandRef = useRef(false);
  useEffect(() => {
    if (didInitialExpandRef.current) return;
    if (servers.length === 0) return;
    didInitialExpandRef.current = true;
    setExpandedServerIds(new Set([servers[0].id]));
    setSelectedServer(servers[0].id);
  }, [servers, setSelectedServer]);

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

  const activeChannelOverride = selectedTextChannel
    ? channelOverrides[selectedTextChannel.id] || {}
    : {};

  const activeTextChannel = selectedTextChannel
    ? {
        id: selectedTextChannel.id,
        name: activeChannelOverride.name ?? selectedTextChannel.name,
        topic: activeChannelOverride.topic,
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

  const previewParticipants: ChannelParticipant[] = React.useMemo(() => {
    if (!selectedVoiceChannelNotJoined) return [];
    const members =
      voicePresenceByChannel[selectedVoiceChannelNotJoined.id] || [];
    return members.map((m) => ({ id: m.id, name: m.name }));
  }, [selectedVoiceChannelNotJoined, voicePresenceByChannel]);

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
  const showMembersForChannel =
    onTextChannelActive && !!activeTextChannel && !!membersOpenById[activeTextChannel.id];

  const handleToggleServer = (serverId: string) => {
    setExpandedServerIds((prev) => {
      const next = new Set(prev);
      if (next.has(serverId)) next.delete(serverId);
      else next.add(serverId);
      return next;
    });
  };

  const handleSelectServer = (serverId: string) => {
    setSelectedServer(serverId);
    setSelectedDm(null);
    const firstText = channelsByServer[serverId]?.find((c) => c.kind === "text");
    setSelectedChannel(firstText ? firstText.id : null);
  };

  const finalizeJoin = (server: Server, channel: Channel, password?: string) => {
    setSelectedServer(server.id);
    setSelectedChannel(channel.id);
    setJoinedChannel({ channel, serverId: server.id, password });
    setSelectedDm(null);
    setVoiceJoinFailed(false);
    setVoiceJoinAttempt((n) => n + 1);
  };

  const handleSelectChannel = (server: Server, channel: Channel) => {
    if (channel.kind === "text") {
      setSelectedServer(server.id);
      setSelectedChannel(channel.id);
      setSelectedDm(null);
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

  const handleSelectDm = (dm: MockDirect) => {
    setSelectedDm(dm);
    setSelectedChannel(null);
  };

  const handleGoToServer = async (serverId: string) => {
    await fetchServers();
    setExpandedServerIds((prev) => {
      const next = new Set(prev);
      next.add(serverId);
      return next;
    });
    setSelectedServer(serverId);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  const handleCreateServer = async (
    name: string,
    description?: string,
    accentColor?: string,
  ) => {
    const server = await createServer(name, description, accentColor);
    try {
      await createChannel(server.id, "general", "text", "public");
    } catch (err) {
      console.error("[ServersPage] failed to create default text channel:", err);
    }
    try {
      await createChannel(server.id, "voice", "voice", "public");
    } catch (err) {
      console.error("[ServersPage] failed to create default voice channel:", err);
    }
    setExpandedServerIds((prev) => {
      const next = new Set(prev);
      next.add(server.id);
      return next;
    });
    setSelectedServer(server.id);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  const handleCreateChannel = async (
    name: string,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password?: string,
  ) => {
    if (channelModalServerId) {
      await createChannel(
        channelModalServerId,
        name,
        kind,
        visibility,
        password,
      );
    }
  };

  const handleDeleteServer = async () => {
    if (deleteConfirm?.type !== "server") return;
    try {
      await deleteServer(deleteConfirm.id);
      if (selectedServerId === deleteConfirm.id) {
        setSelectedServer(null);
        clearChannels(deleteConfirm.id);
      }
      if (joinedChannel?.serverId === deleteConfirm.id) {
        setJoinedChannel(null);
        setVoiceJoinFailed(false);
      }
      setExpandedServerIds((prev) => {
        if (!prev.has(deleteConfirm.id)) return prev;
        const next = new Set(prev);
        next.delete(deleteConfirm.id);
        return next;
      });
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to delete server:", err);
    }
  };

  const handleLeaveServer = async () => {
    if (deleteConfirm?.type !== "leave-server") return;
    try {
      await serverAPI.leave(deleteConfirm.id);
      if (selectedServerId === deleteConfirm.id) {
        setSelectedServer(null);
        clearChannels(deleteConfirm.id);
      }
      if (joinedChannel?.serverId === deleteConfirm.id) {
        setJoinedChannel(null);
        setVoiceJoinFailed(false);
      }
      setExpandedServerIds((prev) => {
        if (!prev.has(deleteConfirm.id)) return prev;
        const next = new Set(prev);
        next.delete(deleteConfirm.id);
        return next;
      });
      await fetchServers();
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to leave server:", err);
    }
  };

  const handleDeleteChannel = async () => {
    if (deleteConfirm?.type !== "channel" || !deleteConfirm.serverId) return;
    try {
      await deleteChannel(deleteConfirm.serverId, deleteConfirm.id);
      if (selectedChannelId === deleteConfirm.id) {
        setSelectedChannel(null);
      }
      if (joinedChannel?.channel.id === deleteConfirm.id) {
        setJoinedChannel(null);
        setVoiceJoinFailed(false);
      }
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to delete channel:", err);
    }
  };

  const handleRemoveFriend = async () => {
    if (deleteConfirm?.type !== "friend") return;
    try {
      await useFriendshipStore.getState().remove(deleteConfirm.id);
      if (selectedDm?.id === deleteConfirm.id) setSelectedDm(null);
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to remove friend:", err);
    }
  };

  const handlePickDmFromModal = (person: MockMember) => {
    const existing = directs.find((d) => d.name === person.name);
    if (existing) {
      setSelectedDm(existing);
      setIsDmModalOpen(false);
      return;
    }
    const newDm: MockDirect = {
      id: "dm-" + Date.now().toString(36),
      name: person.name,
      initials: person.initials,
      color: person.color,
      online: person.status === "online",
      unread: 0,
    };
    setDirects((prev) => [newDm, ...prev]);
    setSelectedDm(newDm);
    setIsDmModalOpen(false);
  };

  const handleVoiceLeave = useCallback(() => {
    setJoinedChannel(null);
    setVoiceJoinFailed(false);
  }, []);

  const handleVoiceJoinFailed = useCallback(() => {
    setVoiceJoinFailed(true);
  }, []);

  const handleLogout = async () => {
    serverAPI.removeAuthToken();
    channelAPI.removeAuthToken();
    friendshipAPI.removeAuthToken();
    dmAPI.removeAuthToken();
    await signOut();
  };

  const error = serversError || channelsError;
  const clearError = () => {
    clearServersError();
    clearChannelsError();
  };

  const userName = user?.name || "You";
  const userStatus = "Online";

  const voiceMembersByChannel: Record<
    string,
    {
      id: string;
      name: string;
      initials: string;
      color: string;
      isMuted?: boolean;
      isSharingScreen?: boolean;
    }[]
  > = React.useMemo(() => {
    const out: Record<
      string,
      {
        id: string;
        name: string;
        initials: string;
        color: string;
        isMuted?: boolean;
        isSharingScreen?: boolean;
      }[]
    > = {};
    for (const [channelId, members] of Object.entries(voicePresenceByChannel)) {
      out[channelId] = members.map((m) => ({
        id: m.id,
        name: m.name,
        initials: initialFor(m.name),
        color: colorForId(m.id),
        isMuted: m.isMuted,
      }));
    }
    return out;
  }, [voicePresenceByChannel]);

  const winTitle = selectedDm
    ? selectedDm.name
    : joinedChannel
      ? `${servers.find((s) => s.id === joinedChannel.serverId)?.name ?? ""} — ${joinedChannel.channel.name}`
      : selectedServer
        ? activeTextChannel
          ? `${selectedServer.name} — ${activeTextChannel.name}`
          : selectedServer.name
        : "Rift";

  return (
    <div className="win">
      <div className="titlebar">
        <div className="traffic">
          <i />
          <i />
          <i />
        </div>
        <div className="win-title">{winTitle}</div>
        <div className="spacer" />
      </div>

      <div className="body-area">
        <Sidebar
          servers={servers}
          channelsByServer={channelsByServer}
          expandedServerIds={expandedServerIds}
          selectedChannelId={selectedChannelId}
          joinedChannelId={joinedChannel?.channel.id ?? null}
          voiceMembers={
            joinedChannel
              ? [
                  {
                    id: user?.id || "you",
                    name: userName || "You",
                    initials: initialFor(userName),
                    color: colorForId(user?.id || "you"),
                  },
                  ...voiceParticipants.map((p) => ({
                    id: p.identity,
                    name: p.name || p.identity,
                    initials: initialFor(p.name || p.identity),
                    color: colorForId(p.identity),
                    isSharingScreen: p.isSharingScreen,
                  })),
                ]
              : []
          }
          voiceMembersByChannel={voiceMembersByChannel}
          selectedDmId={selectedDm?.id ?? null}
          directs={acceptedFriends}
          pendingCount={pendingCount}
          onToggleServer={handleToggleServer}
          onSelectServer={handleSelectServer}
          onSelectChannel={handleSelectChannel}
          onSelectDm={handleSelectDm}
          onAddServer={() => setIsServerModalOpen(true)}
          onAddChannel={(server) => {
            setChannelModalServerId(server.id);
            setIsChannelModalOpen(true);
          }}
          onInviteServer={(server) => setInviteModalServer(server)}
          onAddDm={() => setIsAddFriendOpen(true)}
          onOpenPendingRequests={() => setIsPendingOpen(true)}
          onDeleteServer={(server) => {
            const isOwner = server.ownerId === user?.id;
            setDeleteConfirm({
              type: isOwner ? "server" : "leave-server",
              id: server.id,
              name: server.name,
            });
          }}
          onDeleteChannel={(server, channel) =>
            setDeleteConfirm({
              type: "channel",
              id: channel.id,
              serverId: server.id,
              name: channel.name,
            })
          }
          onOpenSettings={() => setSettingsOpen(true)}
          currentUserId={user?.id}
          currentUserName={userName}
          currentUserStatus={userStatus}
          searchQuery={sidebarQuery}
          onSearchQueryChange={setSidebarQuery}
        />

        <main className="main">
          <div className="main-inner">
            {!selectedServer && !selectedDm ? (
              <Welcome
                loading={serversLoading && servers.length === 0}
                onCreate={() => setIsServerModalOpen(true)}
              />
            ) : selectedDm ? (
              <ChatView
                dm={{
                  friendId: selectedDm.id,
                  name: selectedDm.name,
                  initials: selectedDm.initials,
                  color: selectedDm.color,
                  online: selectedDm.online,
                }}
                currentUserId={user?.id}
                servers={servers}
                onGoToServer={handleGoToServer}
                onRemoveFriend={() =>
                  setDeleteConfirm({
                    type: "friend",
                    id: selectedDm.id,
                    name: selectedDm.name,
                  })
                }
              />
            ) : joinedChannel && selectedChannelId === joinedChannel.channel.id ? (
              <div className="voice-stage-wrap">
                <header className="main-header">
                  <span className="ch-ico">
                    <Icon name="volume" size={16} />
                  </span>
                  <span className="ch-name">{joinedChannel.channel.name}</span>
                  <span className="sep" />
                  <span className="topic voice-stage-status">
                    <span
                      className={`voice-stage-dot ${qualityClass(localQuality)}`}
                      title={`Connection: ${qualityLabel(localQuality)}`}
                    />
                    {voiceJoinFailed ? "Couldn't connect" : "Connected"}
                    {selectedServer ? ` · ${selectedServer.name}` : ""}
                  </span>
                  <div className="right">
                    <button
                      type="button"
                      className="icon-btn"
                      title="Channel settings"
                      onClick={() =>
                        setEditingChannel({
                          id: joinedChannel.channel.id,
                          serverId: joinedChannel.serverId,
                          name: joinedChannel.channel.name,
                        })
                      }
                    >
                      <Icon name="settings" size={15} />
                    </button>
                  </div>
                </header>
                {(() => {
                  const tilesById = new Map<
                    string,
                    {
                      id: string;
                      name: string;
                      initials: string;
                      color: string;
                    }
                  >();
                  const selfId = user?.id || "you";
                  tilesById.set(selfId, {
                    id: selfId,
                    name: userName || "You",
                    initials: initialFor(userName),
                    color: colorForId(selfId),
                  });
                  const presenceMembers = joinedChannel
                    ? voicePresenceByChannel[joinedChannel.channel.id] || []
                    : [];
                  for (const m of presenceMembers) {
                    if (tilesById.has(m.id)) continue;
                    tilesById.set(m.id, {
                      id: m.id,
                      name: m.name,
                      initials: initialFor(m.name),
                      color: colorForId(m.id),
                    });
                  }
                  for (const p of voiceParticipants) {
                    if (tilesById.has(p.identity)) continue;
                    tilesById.set(p.identity, {
                      id: p.identity,
                      name: p.name || p.identity,
                      initials: initialFor(p.name || p.identity),
                      color: colorForId(p.identity),
                    });
                  }
                  const tiles = Array.from(tilesById.values());
                  const cols =
                    tiles.length === 1
                      ? 1
                      : tiles.length === 2
                        ? 2
                        : tiles.length <= 4
                          ? 2
                          : tiles.length <= 6
                            ? 3
                            : tiles.length <= 9
                              ? 3
                              : 4;
                  return (
                    <div className="voice-room">
                      <div
                        className="voice-room-grid"
                        style={
                          {
                            "--vt-cols": cols,
                          } as React.CSSProperties
                        }
                      >
                        {tiles.map((t) => {
                          const isSelf = t.id === (user?.id || "you");
                          const vol = tileVolumes[t.id] ?? 100;
                          return (
                            <div key={t.id} className="voice-tile">
                              <div
                                className="voice-tile-ava"
                                style={{ background: t.color }}
                              >
                                {t.initials}
                              </div>
                              <div className="voice-tile-name">{t.name}</div>
                              {!isSelf && (
                                <div className="voice-tile-volume-wrap">
                                  <button
                                    type="button"
                                    className={
                                      "voice-tile-volume-btn" +
                                      (openVolumeFor === t.id ? " on" : "")
                                    }
                                    title={`Volume ${vol}%`}
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      setOpenVolumeFor((prev) =>
                                        prev === t.id ? null : t.id,
                                      );
                                    }}
                                  >
                                    <Icon
                                      name={
                                        vol === 0
                                          ? "speaker-off"
                                          : vol < 40
                                            ? "speaker-lo"
                                            : "speaker-hi"
                                      }
                                      size={14}
                                    />
                                  </button>
                                  {openVolumeFor === t.id && (
                                    <div
                                      className="voice-tile-volume-pop"
                                      ref={volumePopoverRef}
                                      onMouseDown={(e) => e.stopPropagation()}
                                    >
                                      <input
                                        type="range"
                                        min={0}
                                        max={100}
                                        step={1}
                                        value={vol}
                                        onChange={(e) => {
                                          const v = Number(e.target.value);
                                          setTileVolumes((prev) => ({
                                            ...prev,
                                            [t.id]: v,
                                          }));
                                          getLiveKitService().setRemoteVolume(
                                            t.id,
                                            v / 100,
                                          );
                                        }}
                                        style={
                                          {
                                            ["--pct" as unknown as string]: `${vol}%`,
                                          } as React.CSSProperties
                                        }
                                      />
                                      <span className="voice-tile-volume-pct">
                                        {vol}%
                                      </span>
                                    </div>
                                  )}
                                </div>
                              )}
                            </div>
                          );
                        })}
                      </div>
                      <div className="voice-room-actions">
                        <button
                          type="button"
                          className="voice-room-action leave"
                          onClick={handleVoiceLeave}
                        >
                          <Icon name="logout" size={16} />
                          <span>Leave</span>
                        </button>
                      </div>
                    </div>
                  );
                })()}
              </div>
            ) : !selectedServerChannelsLoaded ? (
              <div className="empty-state" style={{ padding: 60 }}>
                <span className="loading-spinner" />
              </div>
            ) : selectedVoiceChannelNotJoined && selectedServer ? (
              <div className="voice-stage-wrap">
                <header className="main-header">
                  <span className="ch-ico">
                    <Icon name="volume" size={16} />
                  </span>
                  <span className="ch-name">
                    {selectedVoiceChannelNotJoined.name}
                  </span>
                  <span className="sep" />
                  <span className="topic">Disconnected</span>
                  <div className="right">
                    <button
                      type="button"
                      className="icon-btn"
                      title="Channel settings"
                      onClick={() =>
                        setEditingChannel({
                          id: selectedVoiceChannelNotJoined.id,
                          serverId: selectedServer.id,
                          name: selectedVoiceChannelNotJoined.name,
                        })
                      }
                    >
                      <Icon name="settings" size={15} />
                    </button>
                  </div>
                </header>
                <div className="voice-preview">
                  {previewParticipants.length === 0 ? (
                    <div className="voice-preview-empty">No one is here yet.</div>
                  ) : (
                    (() => {
                      const sizeClass =
                        previewParticipants.length === 1
                          ? "solo"
                          : previewParticipants.length <= 4
                            ? "row"
                            : "grid";
                      return (
                        <div className={`voice-stage-orbs ${sizeClass}`}>
                          {previewParticipants.map((p) => (
                            <div key={p.id} className="voice-stage-orb-cell">
                              <div
                                className="voice-stage-orb"
                                style={
                                  {
                                    "--orb-color": colorForId(p.id),
                                  } as React.CSSProperties
                                }
                              >
                                <span className="voice-stage-orb-letter">
                                  {initialFor(p.name)}
                                </span>
                              </div>
                              <div className="voice-stage-orb-name">
                                {p.name}
                              </div>
                            </div>
                          ))}
                        </div>
                      );
                    })()
                  )}
                  <button
                    type="button"
                    className="btn primary voice-preview-join"
                    onClick={() => {
                      if (selectedVoiceChannelNotJoined.visibility === "private") {
                        setPasswordPrompt({
                          server: selectedServer,
                          channel: selectedVoiceChannelNotJoined,
                        });
                        return;
                      }
                      finalizeJoin(selectedServer, selectedVoiceChannelNotJoined);
                    }}
                  >
                    Connect
                  </button>
                </div>
              </div>
            ) : !serverHasTextChannel ? (
              <NoTextChannel
                serverName={selectedServer?.name ?? ""}
                onCreate={() => {
                  setChannelModalServerId(selectedServer?.id ?? null);
                  setIsChannelModalOpen(true);
                }}
              />
            ) : (
              <ChatView
                channel={
                  activeTextChannel
                    ? {
                        id: activeTextChannel.id,
                        name: activeTextChannel.name,
                        topic: activeTextChannel.topic,
                        type: "text",
                      }
                    : undefined
                }
                serverId={selectedServer?.id}
                currentUserId={user?.id}
                membersOpen={
                  !!activeTextChannel ? !!membersOpenById[activeTextChannel.id] : false
                }
                onToggleMembers={
                  activeTextChannel
                    ? () =>
                        setMembersOpenById((prev) => ({
                          ...prev,
                          [activeTextChannel.id]: !prev[activeTextChannel.id],
                        }))
                    : undefined
                }
                onOpenChannelSettings={
                  selectedTextChannel && selectedServer
                    ? () =>
                        setEditingChannel({
                          id: selectedTextChannel.id,
                          serverId: selectedServer.id,
                          name: activeChannelOverride.name ?? selectedTextChannel.name,
                          topic: activeChannelOverride.topic,
                        })
                    : undefined
                }
              />
            )}

            {joinedChannel && (
              <VoiceView
                key={`${joinedChannel.channel.id}-${voiceJoinAttempt}`}
                channelId={joinedChannel.channel.id}
                serverId={joinedChannel.serverId}
                channelName={joinedChannel.channel.name}
                password={joinedChannel.password}
                onLeave={handleVoiceLeave}
                onJoinFailed={handleVoiceJoinFailed}
              />
            )}
          </div>

          {showMembersForChannel && selectedServer && activeTextChannel && (
            <MembersPanel
              serverId={selectedServer.id}
              channelId={activeTextChannel.id}
            />
          )}
        </main>

        {settingsOpen && (
          <SettingsView
            open={settingsOpen}
            theme={theme}
            onTheme={setTheme}
            accent={accent}
            onAccent={setAccent}
            onClose={() => setSettingsOpen(false)}
            onSignOut={handleLogout}
          />
        )}
      </div>


      <CreateServerModal
        isOpen={isServerModalOpen}
        onClose={() => setIsServerModalOpen(false)}
        onCreate={handleCreateServer}
      />

      <CreateChannelModal
        isOpen={isChannelModalOpen}
        onClose={() => {
          setIsChannelModalOpen(false);
          setChannelModalServerId(null);
        }}
        onCreate={handleCreateChannel}
      />

      <PasswordPromptModal
        isOpen={!!passwordPrompt}
        channelName={passwordPrompt?.channel.name ?? ""}
        onClose={() => setPasswordPrompt(null)}
        onSubmit={(password) => {
          if (passwordPrompt) {
            finalizeJoin(passwordPrompt.server, passwordPrompt.channel, password);
            setPasswordPrompt(null);
          }
        }}
      />

      <NewDmModal
        isOpen={isDmModalOpen}
        existingDirects={directs}
        onClose={() => setIsDmModalOpen(false)}
        onPick={handlePickDmFromModal}
      />

      <AddFriendModal
        isOpen={isAddFriendOpen}
        onClose={() => setIsAddFriendOpen(false)}
      />

      <PendingFriendsModal
        isOpen={isPendingOpen}
        onClose={() => setIsPendingOpen(false)}
      />

      <InviteFriendsModal
        server={inviteModalServer}
        friends={friends.filter((f) => f.status === "accepted")}
        onClose={() => setInviteModalServer(null)}
      />

      {editingChannel && (
        <ChannelSettingsModal
          isOpen={!!editingChannel}
          channel={editingChannel}
          onClose={() => setEditingChannel(null)}
          onSave={(patch) => {
            setChannelOverrides((prev) => ({
              ...prev,
              [editingChannel.id]: { ...prev[editingChannel.id], ...patch },
            }));
            setEditingChannel(null);
          }}
          onDelete={() => {
            setDeleteConfirm({
              type: "channel",
              id: editingChannel.id,
              serverId: editingChannel.serverId,
              name: editingChannel.name,
            });
            setEditingChannel(null);
          }}
        />
      )}

      {deleteConfirm && (
        <ConfirmModal
          title={
            deleteConfirm.type === "server"
              ? "Delete server?"
              : deleteConfirm.type === "leave-server"
                ? "Leave server?"
                : deleteConfirm.type === "channel"
                  ? "Delete channel?"
                  : "Remove friend?"
          }
          subtitle={
            deleteConfirm.type === "leave-server"
              ? "You can rejoin if invited again."
              : deleteConfirm.type === "friend"
                ? "You can send a new request later."
                : "This can't be undone."
          }
          targetName={deleteConfirm.name}
          confirmLabel={
            deleteConfirm.type === "leave-server"
              ? "Leave"
              : deleteConfirm.type === "friend"
                ? "Remove"
                : "Delete"
          }
          danger={deleteConfirm.type !== "leave-server"}
          onCancel={() => setDeleteConfirm(null)}
          onConfirm={
            deleteConfirm.type === "server"
              ? handleDeleteServer
              : deleteConfirm.type === "leave-server"
                ? handleLeaveServer
                : deleteConfirm.type === "channel"
                  ? handleDeleteChannel
                  : handleRemoveFriend
          }
        />
      )}

      {error && (
        <div
          style={{
            position: "fixed",
            bottom: 24,
            left: "50%",
            transform: "translateX(-50%)",
            background: "var(--danger)",
            color: "#fff",
            padding: "10px 16px",
            borderRadius: 12,
            display: "flex",
            alignItems: "center",
            gap: 12,
            boxShadow: "var(--shadow-md)",
            zIndex: 1200,
            fontSize: 13,
          }}
        >
          <span>{error}</span>
          <button
            type="button"
            onClick={clearError}
            style={{
              color: "#fff",
              opacity: 0.9,
              width: 22,
              height: 22,
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "center",
              borderRadius: 6,
              background: "transparent",
              border: "none",
              cursor: "pointer",
            }}
            aria-label="Dismiss"
          >
            <Icon name="x" size={12} />
          </button>
        </div>
      )}
    </div>
  );
};

interface WelcomeProps {
  loading: boolean;
  onCreate: () => void;
}

const Welcome: React.FC<WelcomeProps> = ({ loading, onCreate }) => (
  <div className="empty-state" style={{ padding: 60 }}>
    <div className="icon-wrap" style={{ width: 64, height: 64 }}>
      <Icon name="hash" size={26} />
    </div>
    <h3>Welcome to Rift</h3>
    <p>Pick a server on the left or create one to start talking.</p>
    <div style={{ marginTop: 20 }}>
      {loading ? (
        <span className="loading-spinner" />
      ) : (
        <button
          type="button"
          className="btn primary"
          onClick={onCreate}
          style={{ height: 38, padding: "0 18px", whiteSpace: "nowrap" }}
        >
          <Icon name="plus" size={14} />
          Create a server
        </button>
      )}
    </div>
  </div>
);

interface NoTextChannelProps {
  serverName: string;
  onCreate: () => void;
}

const NoTextChannel: React.FC<NoTextChannelProps> = ({ serverName, onCreate }) => (
  <div className="empty-state" style={{ padding: 60 }}>
    <div className="icon-wrap" style={{ width: 64, height: 64 }}>
      <Icon name="hash" size={26} />
    </div>
    <h3>No text channels yet</h3>
    <p>
      {serverName
        ? `${serverName} doesn't have any text channels. Create one to start chatting.`
        : "Create a text channel to start chatting."}
    </p>
    <div style={{ marginTop: 20 }}>
      <button
        type="button"
        className="btn primary"
        onClick={onCreate}
        style={{ height: 38, padding: "0 18px", whiteSpace: "nowrap" }}
      >
        <Icon name="plus" size={14} />
        Create channel
      </button>
    </div>
  </div>
);

export default ServersPage;
