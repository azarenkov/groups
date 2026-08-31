import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAuthStore } from "../store/authStore";
import { useServerStore } from "../store/serverStore";
import { useChannelStore } from "../store/channelStore";
import { useThemeStore } from "../store/themeStore";
import { serverAPI } from "../api/server";
import { callAPI } from "../api/call";
import { attachmentAPI } from "../api/attachment";
import { channelAPI } from "../api/channel";
import CreateServerModal from "../components/CreateServerModal";
import CreateChannelModal from "../components/CreateChannelModal";
import PasswordPromptModal from "../components/PasswordPromptModal";
import NewDmModal from "../components/NewDmModal";
import AddFriendModal from "../components/AddFriendModal";
import UserProfileModal from "../components/rift/UserProfileModal";
import PendingFriendsModal from "../components/PendingFriendsModal";
import InviteFriendsModal from "../components/InviteFriendsModal";
import { useFriendshipStore } from "../store/friendshipStore";
import { useVoicePresenceStore } from "../store/voicePresenceStore";
import {
  useStreamViewersStore,
  streamViewerKey,
} from "../store/streamViewersStore";
import { getLiveKitService } from "../services/livekit";
import { ws } from "../services/websocket";
import { setActiveChatView } from "../services/websocketBootstrap";
import { useUnreadStore } from "../store/unreadStore";
import { usePresenceStore } from "../store/presenceStore";
import { useUserNamesStore } from "../store/userNamesStore";
import { useDmPreviewStore } from "../store/dmPreviewStore";
import type { DmPreview } from "../store/dmPreviewStore";
import { useCallStore } from "../store/callStore";
import { userAPI } from "../api/user";
import { friendshipAPI } from "../api/friendship";
import { dmAPI } from "../api/dm";
import { homeAPI } from "../api/home";
import { scheduleHomeRefresh, useHomeStore } from "../store/homeStore";
import HomeView from "../components/rift/HomeView";
import { colorForId, initialFor } from "../utils/avatar";
import { UserAvatar } from "../components/rift/Avatar";
import { bustAvatar } from "../utils/avatarCache";
import { ApiError } from "../utils/apiError";
import i18n from "../i18n";
import ConfirmModal from "../components/rift/ConfirmModal";
import IncomingCallModal from "../components/rift/IncomingCallModal";
import ActiveCallBar from "../components/rift/ActiveCallBar";
import Sidebar from "../components/rift/Sidebar";
import ChatView from "../components/rift/ChatView";
import VoiceView from "../components/rift/VoiceView";
import CallStage from "../components/rift/CallStage";
import CallReturnBar, { CallErrorToast } from "../components/rift/CallReturnBar";
import VoiceRoomSplash from "../components/rift/VoiceRoomSplash";
import MembersPanel from "../components/rift/MembersPanel";
import SettingsView from "../components/settings/SettingsView";
import ServerSettingsView from "../components/rift/ServerSettingsView";
import Icon from "../components/rift/Icon";
import { useLocalQuality } from "../hooks/useLocalQuality";
import { useParticipants } from "../hooks/useParticipants";
import { qualityClass, qualityLabel } from "../utils/quality";
import { Channel, ChannelKind, ChannelParticipant } from "../types/channel";
import { Server } from "../types/server";
import {
  type MockDirect,
  type MockMember,
} from "../data/mockData";
import "../styles/rift.css";

const ServersPage: React.FC = () => {
  const { t: tModals } = useTranslation("modals");
  const { t: tVoice } = useTranslation("voice");
  const { t: tSidebar } = useTranslation("sidebar");
  const localQuality = useLocalQuality();
  const voiceParticipants = useParticipants();
  const voicePresenceByChannel = useVoicePresenceStore((s) => s.byChannel);
  const streamViewerCounts = useStreamViewersStore((s) => s.counts);
  const unreadByChannel = useUnreadStore((s) => s.byChannel);
  const unreadByDm = useUnreadStore((s) => s.byDm);
  const channelToServer = useUnreadStore((s) => s.channelToServer);
  const { tokens, user, signOut } = useAuthStore();
  const {
    servers,
    selectedServerId,
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
  const [serverModalView, setServerModalView] = useState<
    "choose" | "create" | "join"
  >("choose");
  const [isDmModalOpen, setIsDmModalOpen] = useState(false);
  const [isAddFriendOpen, setIsAddFriendOpen] = useState(false);
  const [isPendingOpen, setIsPendingOpen] = useState(false);
  const [inviteModalServer, setInviteModalServer] = useState<Server | null>(null);
  const [serverSettings, setServerSettings] = useState<Server | null>(null);
  const [channelModal, setChannelModal] = useState<{ serverId: string; kind: ChannelKind } | null>(null);
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

  const onlineUsers = usePresenceStore((s) => s.online);
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const dmPreviews = useDmPreviewStore((s) => s.byFriend);
  const previewText = React.useCallback(
    (p: DmPreview | undefined) => {
      if (!p) return undefined;
      const body = p.callStatus
        ? tSidebar("previewCall")
        : p.inviteServerId
          ? tSidebar("previewInvite")
          : p.content ||
            (p.hasAttachments ? tSidebar("previewAttachment") : "");
      return p.senderId === user?.id ? `${tSidebar("previewYou")} ${body}` : body;
    },
    [tSidebar, user?.id],
  );
  const acceptedFriends: MockDirect[] = React.useMemo(() => {
    const items: MockDirect[] = friends
      .filter((f) => f.status === "accepted")
      .map((f) => ({
        id: f.friendId,
        name: f.friendName,
        initials: initialFor(f.friendName),
        color: colorForId(f.friendId),
        online: !!onlineUsers[f.friendId],
        unread: 0,
        lastMessage: previewText(dmPreviews[f.friendId]),
        isFriend: true,
        friendship: "accepted" as const,
      }));
    const known = new Set(items.map((d) => d.id));
    const pendingById = new Map(
      friends.filter((f) => f.status !== "accepted").map((f) => [f.friendId, f] as const),
    );
    for (const [peerId, p] of Object.entries(dmPreviews)) {
      if (known.has(peerId)) continue;
      const pending = pendingById.get(peerId);
      const name =
        nameOverrides[peerId] ??
        pending?.friendName ??
        p.peerName ??
        (selectedDm?.id === peerId ? selectedDm.name : undefined);
      if (!name) continue;
      items.push({
        id: peerId,
        name,
        initials: initialFor(name),
        color: colorForId(peerId),
        online: false,
        unread: 0,
        lastMessage: previewText(p),
        isFriend: false,
        friendship:
          pending?.status === "pending_outgoing" || pending?.status === "pending_incoming"
            ? pending.status
            : "none",
      });
    }
    return items;
  }, [friends, onlineUsers, dmPreviews, previewText, nameOverrides, selectedDm]);

  useEffect(() => {
    if (!selectedDm) return;
    const fresh = acceptedFriends.find((d) => d.id === selectedDm.id);
    if (!fresh) {
      if (selectedDm.friendship !== "none" && selectedDm.friendship !== "pending_outgoing" && selectedDm.friendship !== "pending_incoming") {
        setSelectedDm(null);
        return;
      }
      const pending = friends.find((f) => f.friendId === selectedDm.id);
      const status =
        pending?.status === "pending_outgoing" || pending?.status === "pending_incoming"
          ? pending.status
          : "none";
      if (status !== selectedDm.friendship) {
        setSelectedDm((prev) => (prev && prev.id === selectedDm.id ? { ...prev, friendship: status } : prev));
      }
      return;
    }
    if (
      fresh.name === selectedDm.name &&
      fresh.isFriend === selectedDm.isFriend &&
      fresh.friendship === selectedDm.friendship
    )
      return;
    setSelectedDm((prev) =>
      prev && prev.id === fresh.id
        ? { ...prev, name: fresh.name, initials: fresh.initials, isFriend: fresh.isFriend, friendship: fresh.friendship }
        : prev,
    );
  }, [acceptedFriends, selectedDm, friends]);

  const pendingCount = React.useMemo(
    () =>
      friends.filter((f) => f.status === "pending_incoming").length,
    [friends],
  );

  const [membersOpenById, setMembersOpenById] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (tokens?.accessToken) {
      serverAPI.setAuthToken(tokens.accessToken);
      channelAPI.setAuthToken(tokens.accessToken);
      friendshipAPI.setAuthToken(tokens.accessToken);
      dmAPI.setAuthToken(tokens.accessToken);
      homeAPI.setAuthToken(tokens.accessToken);
      callAPI.setAuthToken(tokens.accessToken);
      attachmentAPI.setAuthToken(tokens.accessToken);
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
      useVoicePresenceStore.getState().resetRosters();
      usePresenceStore.getState().clearAll();
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
        .then(({ items, streamViewers }) => {
          const groups: Record<
            string,
            {
              id: string;
              name: string;
              isMuted: boolean;
              mutedBy?: string;
              cameraOn: boolean;
              screenOn: boolean;
            }[]
          > = {};
          for (const item of items) {
            (groups[item.channelId] ||= []).push({
              id: item.userId,
              name: item.userName,
              isMuted: item.isMuted,
              mutedBy: item.mutedBy ?? undefined,
              cameraOn: item.cameraOn,
              screenOn: item.screenOn,
            });
          }
          const snapshotChannels = new Set(Object.keys(groups));
          const knownChannels =
            useChannelStore.getState().channelsByServer[s.id] || [];
          for (const c of knownChannels) {
            if (c.kind !== "voice") continue;
            if (snapshotChannels.has(c.id)) continue;
            useVoicePresenceStore.getState().clearChannel(c.id);
          }
          for (const [channelId, members] of Object.entries(groups)) {
            useVoicePresenceStore.getState().setMembers(channelId, members);
          }
          useStreamViewersStore.getState().resetForServer(s.id, streamViewers);
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

  const callStageShown = useCallStore((s) => !!s.activeCallId && s.focused);
  const [windowFocused, setWindowFocused] = useState(
    () => typeof document === "undefined" || document.hasFocus(),
  );
  useEffect(() => {
    const update = () =>
      setWindowFocused(document.hasFocus() && !document.hidden);
    window.addEventListener("focus", update);
    window.addEventListener("blur", update);
    document.addEventListener("visibilitychange", update);
    return () => {
      window.removeEventListener("focus", update);
      window.removeEventListener("blur", update);
      document.removeEventListener("visibilitychange", update);
    };
  }, []);
  const chatHidden =
    !windowFocused || settingsOpen || !!serverSettings || callStageShown;
  const visibleTextChannelId =
    !chatHidden && onTextChannelActive && activeTextChannel
      ? activeTextChannel.id
      : null;
  const visibleDmId = !chatHidden && selectedDm ? selectedDm.id : null;
  useEffect(() => {
    if (visibleDmId) {
      setActiveChatView({ kind: "dm", id: visibleDmId });
      if ((useUnreadStore.getState().byDm[visibleDmId] ?? 0) > 0) {
        useUnreadStore.getState().resetDm(visibleDmId);
        userAPI.markDmRead(visibleDmId).catch((err) => {
          console.warn("[read-state] mark dm read failed:", err);
        });
      }
    } else if (visibleTextChannelId) {
      setActiveChatView({ kind: "channel", id: visibleTextChannelId });
      if ((useUnreadStore.getState().byChannel[visibleTextChannelId] ?? 0) > 0) {
        useUnreadStore.getState().resetChannel(visibleTextChannelId);
        userAPI.markChannelRead(visibleTextChannelId).catch((err) => {
          console.warn("[read-state] mark channel read failed:", err);
        });
      }
    } else {
      setActiveChatView(null);
    }
    return () => setActiveChatView(null);
  }, [visibleDmId, visibleTextChannelId]);

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
    void useCallStore.getState().leave(true);
    setSelectedServer(server.id);
    setSelectedChannel(channel.id);
    setJoinedChannel({ channel, serverId: server.id, password });
    setSelectedDm(null);
    setVoiceJoinFailed(false);
    setVoiceJoinAttempt((n) => n + 1);
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

  const handleSelectDm = (dm: MockDirect) => {
    useCallStore.getState().setFocused(false);
    setSelectedDm(dm);
    setSelectedChannel(null);
    useUnreadStore.getState().resetDm(dm.id);
    userAPI.markDmRead(dm.id).catch((err) => {
      console.warn("[read-state] mark dm read failed:", err);
    });
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
    logoFile?: File,
  ): Promise<void | { logoError?: string }> => {
    const server = await createServer(name, description, accentColor);
    let logoError: string | undefined;
    if (logoFile) {
      try {
        await serverAPI.uploadLogo(server.id, logoFile);
        bustAvatar("server", server.id);
      } catch (err) {
        console.error("[ServersPage] failed to upload server logo:", err);
        const apiErr = err instanceof ApiError ? err : null;
        const reason =
          apiErr?.code === "IMAGE_CONTENT_REJECTED"
            ? tModals("serverModal.logoRejected")
            : apiErr?.message || tModals("serverModal.logoUploadGeneric");
        logoError = tModals("serverModal.logoUploadFailed", { reason });
      }
    }
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
    if (logoError) return { logoError };
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
      setServerSettings((prev) => (prev?.id === deleteConfirm.id ? null : prev));
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
      setServerSettings((prev) => (prev?.id === deleteConfirm.id ? null : prev));
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

  const activeCallId = useCallStore((s) => s.activeCallId);
  const callFocused = useCallStore((s) => s.focused);
  const callPeerId = useCallStore((s) => s.peerId);
  const prevActiveCallRef = useRef<string | null>(null);
  useEffect(() => {
    const prev = prevActiveCallRef.current;
    prevActiveCallRef.current = activeCallId;
    if (!prev && activeCallId && joinedChannel) {
      setJoinedChannel(null);
      setVoiceJoinFailed(false);
    }
  }, [activeCallId, joinedChannel]);

  useEffect(() => {
    if (!callFocused || !activeCallId || !callPeerId) return;
    if (selectedDm?.id === callPeerId) return;
    const peer = acceptedFriends.find((d) => d.id === callPeerId);
    if (!peer) return;
    setSelectedDm(peer);
    setSelectedChannel(null);
    setJoinedChannel(null);
  }, [
    callFocused,
    activeCallId,
    callPeerId,
    selectedDm,
    acceptedFriends,
    setSelectedChannel,
  ]);

  const watchServerId = joinedChannel?.serverId;
  const watchChannelId = joinedChannel?.channel.id;
  const watchStateRef = useRef(
    new Map<string, { desired: boolean; running: boolean }>(),
  );
  const reportStreamWatch = useCallback(
    (streamerId: string, watching: boolean) => {
      if (!watchServerId || !watchChannelId) return;
      const key = `${watchServerId}:${watchChannelId}:${streamerId}`;
      const states = watchStateRef.current;
      const entry = states.get(key) ?? { desired: watching, running: false };
      entry.desired = watching;
      states.set(key, entry);
      if (entry.running) return;
      entry.running = true;
      void (async () => {
        let lastSent: boolean | undefined;
        while (states.get(key) && states.get(key)!.desired !== lastSent) {
          const target = states.get(key)!.desired;
          const sessionId = getLiveKitService().getLocalSessionId();
          if (!sessionId) break;
          try {
            await serverAPI.updateStreamViewer(
              watchServerId,
              watchChannelId,
              streamerId,
              sessionId,
              target,
            );
            lastSent = target;
          } catch (err) {
            console.warn("[stream-viewers] report failed:", err);
            await new Promise((resolve) => setTimeout(resolve, 2000));
          }
        }
        const e = states.get(key);
        if (e) e.running = false;
      })();
    },
    [watchServerId, watchChannelId],
  );

  const handleLogout = async () => {
    if (useCallStore.getState().activeCallId) {
      await useCallStore.getState().leave();
    }
    serverAPI.removeAuthToken();
    channelAPI.removeAuthToken();
    friendshipAPI.removeAuthToken();
    dmAPI.removeAuthToken();
    homeAPI.removeAuthToken();
    callAPI.removeAuthToken();
    await signOut();
  };

  const error = serversError || channelsError;
  const clearError = () => {
    clearServersError();
    clearChannelsError();
  };

  const userName = user?.name || "You";
  const [appearOffline, setAppearOfflineState] = useState<boolean>(() => {
    try {
      return localStorage.getItem("rift:appearOffline") === "1";
    } catch {
      return false;
    }
  });
  const userStatus = appearOffline ? "offline" : "online";
  useEffect(() => {
    if (!tokens?.accessToken) return;
    userAPI.setAppearOffline(appearOffline).catch((err) => {
      console.warn("[presence] setAppearOffline failed:", err);
    });
  }, [appearOffline, tokens?.accessToken]);
  useEffect(() => {
    if (!user?.id) return;
    usePresenceStore.getState().setSelf(user.id, !appearOffline);
  }, [user?.id, appearOffline]);
  const handleSetStatus = useCallback((status: "online" | "offline") => {
    const next = status === "offline";
    setAppearOfflineState(next);
    try {
      localStorage.setItem("rift:appearOffline", next ? "1" : "0");
    } catch {}
  }, []);

  const homeOverview = useHomeStore((s) => s.overview);
  const homeFailed = useHomeStore((s) => s.failed);
  const fetchHome = useHomeStore((s) => s.fetch);
  const homeActive = !selectedServer && !selectedDm;
  useEffect(() => {
    if (!tokens?.accessToken || !homeActive) return;
    void fetchHome();
    const interval = window.setInterval(() => void fetchHome(), 60_000);
    const onFocus = () => void fetchHome();
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener("focus", onFocus);
    };
  }, [tokens?.accessToken, homeActive, fetchHome]);
  const homeRefreshArmedRef = useRef(false);
  useEffect(() => {
    if (!homeActive) {
      homeRefreshArmedRef.current = false;
      return;
    }
    if (!homeRefreshArmedRef.current) {
      homeRefreshArmedRef.current = true;
      return;
    }
    scheduleHomeRefresh();
  }, [homeActive, voicePresenceByChannel, onlineUsers, friends]);

  const handleGoHome = () => {
    useCallStore.getState().setFocused(false);
    setSelectedServer(null);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  const handleJoinFromHome = async (serverId: string, channelId: string) => {
      const server = servers.find((s) => s.id === serverId);
      if (!server) return;
      let channel = channelsByServer[serverId]?.find((c) => c.id === channelId);
      if (!channel) {
        await fetchChannels(serverId);
        channel = useChannelStore
          .getState()
          .channelsByServer[serverId]?.find((c) => c.id === channelId);
      }
      if (!channel) return;
      setExpandedServerIds((prev) => {
        if (prev.has(serverId)) return prev;
        const next = new Set(prev);
        next.add(serverId);
        return next;
      });
      handleSelectChannel(server, channel);
  };

  const handleOpenDmFromHome = (friendId: string, name?: string) => {
    const dm = acceptedFriends.find((d) => d.id === friendId);
    if (dm) {
      handleSelectDm(dm);
      return;
    }
    const preview = dmPreviews[friendId];
    const fallbackName = name ?? nameOverrides[friendId] ?? preview?.peerName;
    if (!fallbackName) return;
    handleSelectDm({
      id: friendId,
      name: fallbackName,
      initials: initialFor(fallbackName),
      color: colorForId(friendId),
      online: false,
      unread: 0,
      isFriend: false,
      friendship: "none",
    });
  };

  const handleCallFromHome = (friendId: string) => {
    handleOpenDmFromHome(friendId);
    const cs = useCallStore.getState();
    const inCallWithFriend =
      !!cs.activeCallId &&
      (cs.peerId === friendId ||
        friendId in cs.participants ||
        cs.ringing.includes(friendId));
    if (inCallWithFriend) {
      cs.setFocused(true);
      return;
    }
    cs.start([friendId]).catch((e) => {
      const apiErr = e instanceof ApiError ? e : null;
      useCallStore.getState().setCallError(
        apiErr?.code === "CALL_NOT_ALLOWED"
          ? i18n.t("callNotAllowed", { ns: "call" })
          : i18n.t("callFailed", { ns: "call" }),
      );
    });
  };

  const voiceMembersByChannel: Record<
    string,
    {
      id: string;
      name: string;
      initials: string;
      color: string;
      isMuted?: boolean;
      mutedBy?: string;
      isSharingScreen?: boolean;
      isCameraOn?: boolean;
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
        mutedBy?: string;
        isSharingScreen?: boolean;
        isCameraOn?: boolean;
      }[]
    > = {};
    for (const [channelId, members] of Object.entries(voicePresenceByChannel)) {
      out[channelId] = members.map((m) => ({
        id: m.id,
        name: m.name,
        initials: initialFor(m.name),
        color: colorForId(m.id),
        isMuted: m.isMuted,
        mutedBy: m.mutedBy,
        isSharingScreen: m.screenOn,
        isCameraOn: m.cameraOn,
      }));
    }
    return out;
  }, [voicePresenceByChannel]);


  return (
    <div className="win">
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
                  ...voiceParticipants.map((p) => {
                    const name =
                      nameOverrides[p.identity] ?? (p.name || p.identity);
                    return {
                      id: p.identity,
                      name,
                      initials: initialFor(name),
                      color: colorForId(p.identity),
                    };
                  }),
                ]
              : []
          }
          voiceMembersByChannel={voiceMembersByChannel}
          selectedDmId={selectedDm?.id ?? null}
          directs={acceptedFriends}
          onToggleServer={handleToggleServer}
          onSelectServer={handleSelectServer}
          onSelectChannel={handleSelectChannel}
          onSelectDm={handleSelectDm}
          onInviteServer={(server) => setInviteModalServer(server)}
          onOpenServerSettings={(server) => setServerSettings(server)}
          onAddChannel={(server, kind) => setChannelModal({ serverId: server.id, kind })}
          onDeleteChannel={(server, channel) =>
            setDeleteConfirm({
              type: "channel",
              id: channel.id,
              serverId: server.id,
              name: channel.name,
            })
          }
          onOpenSettings={() => setSettingsOpen(true)}
          onAddFriend={() => setIsAddFriendOpen(true)}
          onOpenPending={() => setIsPendingOpen(true)}
          onGoHome={handleGoHome}
          homeActive={homeActive}
          pendingCount={pendingCount}
          onCreateServer={() => {
            setServerModalView("create");
            setIsServerModalOpen(true);
          }}
          onJoinServer={() => {
            setServerModalView("join");
            setIsServerModalOpen(true);
          }}
          currentUserId={user?.id}
          currentUserName={userName}
          currentUserStatus={userStatus}
          searchQuery={sidebarQuery}
          onSearchQueryChange={setSidebarQuery}
          unreadByChannel={unreadByChannel}
          unreadByDm={unreadByDm}
          channelToServer={channelToServer}
          onSetStatus={handleSetStatus}
        />

        <main className="main">
          <div className="main-inner">
            <CallReturnBar />
            <CallErrorToast />
            {activeCallId && callFocused && !selectedDm ? (
              <CallStage
                onMinimize={() => useCallStore.getState().setFocused(false)}
              />
            ) : !selectedServer && !selectedDm ? (
              <HomeView
                userName={userName}
                overview={homeOverview}
                overviewFailed={homeFailed}
                onRetry={() => void fetchHome()}
                friends={friends}
                onlineUsers={onlineUsers}
                joinedChannelId={joinedChannel?.channel.id ?? null}
                onJoinRoom={(serverId, channelId) => void handleJoinFromHome(serverId, channelId)}
                onOpenDm={handleOpenDmFromHome}
                onCall={handleCallFromHome}
                onAddFriend={() => setIsAddFriendOpen(true)}
              />
            ) : selectedDm ? (
              <ChatView
                dm={{
                  friendId: selectedDm.id,
                  name: nameOverrides[selectedDm.id] ?? selectedDm.name,
                  initials: selectedDm.initials,
                  color: selectedDm.color,
                  online: selectedDm.isFriend !== false && !!onlineUsers[selectedDm.id],
                  isFriend: selectedDm.isFriend !== false,
                  friendship: selectedDm.friendship ?? "accepted",
                }}
                currentUserId={user?.id}
                servers={servers}
                onGoToServer={handleGoToServer}
                callMode={!!activeCallId && callFocused}
                topSlot={
                  activeCallId && callFocused ? (
                    <CallStage
                      onMinimize={() =>
                        useCallStore.getState().setFocused(false)
                      }
                    />
                  ) : undefined
                }
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
                    {voiceJoinFailed ? tVoice("disconnected") : tVoice("connected")}
                    {selectedServer ? ` · ${selectedServer.name}` : ""}
                  </span>
                </header>
                <div id="voice-stage-slot" className="voice-stage-slot" />
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
                  <span className="topic">{tVoice("disconnected")}</span>
                </header>
                <div className="voice-preview">
                  {previewParticipants.length === 0 ? (
                    <VoiceRoomSplash
                      channelName={selectedVoiceChannelNotJoined.name}
                      onConnect={() => {
                        if (
                          selectedVoiceChannelNotJoined.visibility === "private"
                        ) {
                          setPasswordPrompt({
                            server: selectedServer,
                            channel: selectedVoiceChannelNotJoined,
                          });
                          return;
                        }
                        finalizeJoin(
                          selectedServer,
                          selectedVoiceChannelNotJoined,
                        );
                      }}
                      onInvite={() => setInviteModalServer(selectedServer)}
                    />
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
                              <UserAvatar
                                userId={p.id}
                                name={p.name}
                                className="voice-stage-orb"
                                style={
                                  {
                                    "--orb-color": colorForId(p.id),
                                  } as React.CSSProperties
                                }
                              />
                              <div className="voice-stage-orb-name">
                                {p.name}
                              </div>
                            </div>
                          ))}
                        </div>
                      );
                    })()
                  )}
                  {previewParticipants.length > 0 && (
                    <button
                      type="button"
                      className="btn primary voice-preview-join"
                      onClick={() => {
                        if (
                          selectedVoiceChannelNotJoined.visibility === "private"
                        ) {
                          setPasswordPrompt({
                            server: selectedServer,
                            channel: selectedVoiceChannelNotJoined,
                          });
                          return;
                        }
                        finalizeJoin(
                          selectedServer,
                          selectedVoiceChannelNotJoined,
                        );
                      }}
                    >
                      {tVoice("connect")}
                    </button>
                  )}
                </div>
              </div>
            ) : !serverHasTextChannel ? (
              <NoTextChannel
                serverName={selectedServer?.name ?? ""}
                onCreate={() => {
                  if (selectedServer) {
                    setChannelModal({ serverId: selectedServer.id, kind: "text" });
                  }
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
              />
            )}

            {joinedChannel && (
              <VoiceView
                key={`${joinedChannel.channel.id}-${voiceJoinAttempt}`}
                channelId={joinedChannel.channel.id}
                serverId={joinedChannel.serverId}
                channelName={joinedChannel.channel.name}
                password={joinedChannel.password}
                currentUserId={user?.id}
                userName={userName}
                stageActive={
                  selectedChannelId === joinedChannel.channel.id &&
                  !voiceJoinFailed
                }
                viewersFor={(id) =>
                  streamViewerCounts[
                    streamViewerKey(
                      joinedChannel.serverId,
                      joinedChannel.channel.id,
                      id === (user?.id || "you") ? user?.id || "you" : id,
                    )
                  ]
                }
                onStreamWatch={reportStreamWatch}
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

        {serverSettings && (
          <ServerSettingsView
            server={
              servers.find((s) => s.id === serverSettings.id) ?? serverSettings
            }
            friends={friends}
            channels={channelsByServer[serverSettings.id] ?? []}
            currentUserId={user?.id}
            onClose={() => setServerSettings(null)}
            onDeleteServer={(server) => {
              const isOwner = server.ownerId === user?.id;
              setDeleteConfirm({
                type: isOwner ? "server" : "leave-server",
                id: server.id,
                name: server.name,
              });
            }}
            onCreateChannel={(name, kind, visibility, password) =>
              createChannel(serverSettings.id, name, kind, visibility, password)
            }
            onDeleteChannel={(channel) =>
              setDeleteConfirm({
                type: "channel",
                id: channel.id,
                serverId: serverSettings.id,
                name: channel.name,
              })
            }
          />
        )}
      </div>


      <CreateServerModal
        isOpen={isServerModalOpen}
        onClose={() => setIsServerModalOpen(false)}
        onCreate={handleCreateServer}
        initialView={serverModalView}
        onJoinByCode={async (code) => {
          const server = await serverAPI.redeemInviteCode(code);
          useServerStore.getState().addServer(server);
          setExpandedServerIds((prev) => {
            const next = new Set(prev);
            next.add(server.id);
            return next;
          });
          setSelectedServer(server.id);
          setSelectedChannel(null);
          setSelectedDm(null);
        }}
      />

      <CreateChannelModal
        isOpen={!!channelModal}
        defaultKind={channelModal?.kind}
        onClose={() => setChannelModal(null)}
        onCreate={async (name, kind, visibility, password) => {
          if (!channelModal) return;
          await createChannel(channelModal.serverId, name, kind, visibility, password);
        }}
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

      <IncomingCallModal />
      <ActiveCallBar />

      <UserProfileModal
        onOpenDm={handleOpenDmFromHome}
        onCall={handleCallFromHome}
        onRemoveFriend={(id, name) => setDeleteConfirm({ type: "friend", id, name })}
      />

      <AddFriendModal
        isOpen={isAddFriendOpen}
        onClose={() => setIsAddFriendOpen(false)}
      />

      <PendingFriendsModal
        isOpen={isPendingOpen}
        onClose={() => setIsPendingOpen(false)}
        onAddFriend={() => setIsAddFriendOpen(true)}
      />

      <InviteFriendsModal
        server={inviteModalServer}
        friends={friends.filter((f) => f.status === "accepted")}
        onClose={() => setInviteModalServer(null)}
      />

      {deleteConfirm && (
        <ConfirmModal
          title={
            deleteConfirm.type === "server"
              ? tModals("confirm.deleteServerTitle")
              : deleteConfirm.type === "leave-server"
                ? tModals("confirm.leaveServerTitle")
                : deleteConfirm.type === "channel"
                  ? tModals("confirm.deleteChannelTitle")
                  : tModals("confirm.removeFriendTitle")
          }
          subtitle={
            deleteConfirm.type === "leave-server"
              ? tModals("confirm.leaveServerSubtitle")
              : deleteConfirm.type === "friend"
                ? tModals("confirm.removeFriendSubtitle")
                : tModals("confirm.thisCantBeUndone")
          }
          targetName={deleteConfirm.name}
          confirmLabel={
            deleteConfirm.type === "leave-server"
              ? tModals("confirm.leave")
              : deleteConfirm.type === "friend"
                ? tModals("confirm.remove")
                : tModals("confirm.delete")
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

interface NoTextChannelProps {
  serverName: string;
  onCreate: () => void;
}

const NoTextChannel: React.FC<NoTextChannelProps> = ({ serverName, onCreate }) => {
  const { t } = useTranslation("sidebar");
  return (
    <div className="empty-state" style={{ padding: 60 }}>
      <div className="icon-wrap" style={{ width: 64, height: 64 }}>
        <Icon name="hash" size={26} />
      </div>
      <h3>{t("noTextChannelsTitle")}</h3>
      <p>
        {serverName
          ? t("noTextChannelsBody", { server: serverName })
          : t("noTextChannelsBodyGeneric")}
      </p>
      <div style={{ marginTop: 20 }}>
        <button
          type="button"
          className="btn primary"
          onClick={onCreate}
          style={{ height: 38, padding: "0 18px", whiteSpace: "nowrap" }}
        >
          <Icon name="plus" size={14} />
          {t("createChannelCta")}
        </button>
      </div>
    </div>
  );
};

export default ServersPage;
