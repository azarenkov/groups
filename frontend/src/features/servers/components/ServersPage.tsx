import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { useAuthStore } from "../../auth/store/authStore";
import { useServerStore } from "../store/serverStore";
import { useChannelStore } from "../store/channelStore";
import { serverAPI } from "../api/server";
import { extractInviteToken } from "../../../shared/lib/inviteLinks";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { useBlockStore } from "../../social/store/blockStore";
import { useVoicePresenceStore } from "../../voice/store/voicePresenceStore";
import {
  useStreamViewersStore,
  streamViewerKey,
} from "../../voice/store/streamViewersStore";
import { ws } from "../../../shared/ws/websocket";
import { setActiveChatView } from "../../../shared/ws/websocketBootstrap";
import { useUnreadStore } from "../../chat/store/unreadStore";
import { usePresenceStore } from "../../social/store/presenceStore";
import { useUserNamesStore } from "../../social/store/userNamesStore";
import { useDmPreviewStore } from "../../dm/store/dmPreviewStore";
import type { DmPreview } from "../../dm/store/dmPreviewStore";
import { useCallStore } from "../../voice/store/callStore";
import { userAPI } from "../../social/api/user";
import { useHomeStore } from "../../home/store/homeStore";
import HomeView from "../../home/components/HomeView";
import { colorForId, initialFor } from "../../../shared/lib/avatar";
import { UserAvatar } from "../../../shared/ui/Avatar";
import { bustAvatar } from "../../../shared/lib/avatarCache";
import { ApiError } from "../../../shared/lib/apiError";
import i18n from "../../../i18n";
import Sidebar from "./Sidebar";
import ChatView from "../../chat/components/ChatView";
import VoiceView from "../../voice/components/VoiceView";
import CallStage from "../../voice/components/CallStage";
import CallReturnBar, { CallErrorToast } from "../../voice/components/CallReturnBar";
import VoiceRoomSplash from "../../voice/components/VoiceRoomSplash";
import MembersPanel from "./MembersPanel";
import SettingsView from "../../settings/components/SettingsView";
import { useHotkey } from "../../settings/lib/hotkeys";
import ServerSettingsView from "./ServerSettingsView";
import ModalsHost from "./ModalsHost";
import { useVoiceSession } from "../../voice/hooks/useVoiceSession";
import { useHomeRefresh } from "../../home/hooks/useHomeRefresh";
import ErrorToast from "../../../shared/ui/ErrorToast";
import Icon from "../../../shared/ui/Icon";
import { useLocalQuality } from "../../voice/hooks/useLocalQuality";
import { useParticipants } from "../../voice/hooks/useParticipants";
import { useLocalDeafened } from "../../voice/hooks/useLocalDeafened";
import { qualityClass, qualityLabel } from "../../voice/lib/quality";
import { Channel, ChannelKind, ChannelParticipant } from "../../../shared/types/channel";
import { Server } from "../../../shared/types/server";
import type { DirectConversation } from "../../dm/types";
import "../../../styles/rift.css";

const ServersPage: React.FC = () => {
  const { t: tModals } = useTranslation("modals");
  const { t: tVoice } = useTranslation("voice");
  const { t: tSidebar } = useTranslation("sidebar");
  const localQuality = useLocalQuality();
  const voiceParticipants = useParticipants();
  const selfDeafened = useLocalDeafened();
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

  useHotkey("openSettings", () => setSettingsOpen(true));

  const [isServerModalOpen, setIsServerModalOpen] = useState(false);
  const [serverModalView, setServerModalView] = useState<
    "choose" | "create" | "join"
  >("choose");
  const [joinToken, setJoinToken] = useState<string | null>(null);
  const [searchParams, setSearchParams] = useSearchParams();
  useEffect(() => {
    const raw = searchParams.get("join");
    if (!raw) return;
    setSearchParams({}, { replace: true });
    const token = extractInviteToken(raw);
    if (!token) return;
    setJoinToken(token);
    setServerModalView("join");
    setIsServerModalOpen(true);
  }, [searchParams, setSearchParams]);
  const [isAddFriendOpen, setIsAddFriendOpen] = useState(false);
  const [isPendingOpen, setIsPendingOpen] = useState(false);
  const [inviteModalServer, setInviteModalServer] = useState<Server | null>(null);
  const [serverSettings, setServerSettings] = useState<Server | null>(null);
  const [channelModal, setChannelModal] = useState<{ serverId: string; kind: ChannelKind } | null>(null);
  const friends = useFriendshipStore((s) => s.friends);
  const friendStatuses = useFriendshipStore((s) => s.statusById);
  const friendsNextCursor = useFriendshipStore((s) => s.nextCursor);
  const loadMoreFriends = useFriendshipStore((s) => s.loadMore);
  const previewsNextCursor = useDmPreviewStore((s) => s.nextCursor);
  const loadMorePreviews = useDmPreviewStore((s) => s.loadMore);
  const fetchFriends = useFriendshipStore((s) => s.fetchFriends);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    type: "server" | "leave-server" | "channel" | "friend" | "block";
    id: string;
    serverId?: string;
    name: string;
  } | null>(null);

  const {
    joinedChannel,
    setJoinedChannel,
    voiceJoinFailed,
    setVoiceJoinFailed,
    voiceJoinAttempt,
    passwordPrompt,
    setPasswordPrompt,
    handleVoiceLeave,
    handleVoiceJoinFailed,
    startVoiceJoin,
    reportStreamWatch,
  } = useVoiceSession();
  const [selectedDm, setSelectedDm] = useState<DirectConversation | null>(null);
  const [expandedServerIds, setExpandedServerIds] = useState<Set<string>>(
    () => new Set<string>(),
  );
  const [sidebarQuery, setSidebarQuery] = useState("");

  const onlineUsers = usePresenceStore((s) => s.online);
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const dmPreviews = useDmPreviewStore((s) => s.byFriend);
  const blockedPeers = useBlockStore((s) => s.blocked);
  const blockedByPeers = useBlockStore((s) => s.blockedBy);
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
  const acceptedFriends: DirectConversation[] = React.useMemo(() => {
    const items: DirectConversation[] = friends
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
      if (
        (blockedPeers[peerId] || blockedByPeers[peerId]) &&
        selectedDm?.id !== peerId
      )
        continue;
      const pending = pendingById.get(peerId);
      const name =
        nameOverrides[peerId] ??
        pending?.friendName ??
        p.peerName ??
        (selectedDm?.id === peerId ? selectedDm.name : undefined);
      if (!name) continue;
      const status = friendStatuses[peerId];
      const accepted = status === "accepted";
      items.push({
        id: peerId,
        name,
        initials: initialFor(name),
        color: colorForId(peerId),
        online: accepted && !!onlineUsers[peerId],
        unread: 0,
        lastMessage: previewText(p),
        isFriend: accepted,
        friendship: accepted
          ? "accepted"
          : status === "pending_outgoing" || status === "pending_incoming"
            ? status
            : "none",
      });
    }
    return items;
  }, [
    friends,
    friendStatuses,
    onlineUsers,
    dmPreviews,
    previewText,
    nameOverrides,
    selectedDm,
    blockedPeers,
    blockedByPeers,
  ]);

  useEffect(() => {
    if (!selectedDm) return;
    const fresh = acceptedFriends.find((d) => d.id === selectedDm.id);
    if (!fresh) {
      const known = friendStatuses[selectedDm.id];
      if (known === "accepted") return;
      if (selectedDm.friendship !== "none" && selectedDm.friendship !== "pending_outgoing" && selectedDm.friendship !== "pending_incoming") {
        setSelectedDm(null);
        return;
      }
      const status =
        known === "pending_outgoing" || known === "pending_incoming" ? known : "none";
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
  }, [acceptedFriends, selectedDm, friendStatuses]);

  const pendingCount = React.useMemo(
    () => Object.values(friendStatuses).filter((s) => s === "pending_incoming").length,
    [friendStatuses],
  );

  const [membersOpenById, setMembersOpenById] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (tokens?.accessToken) {
      fetchServers();
      fetchFriends();
      void useBlockStore.getState().fetch();
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
              deafenedBy?: string;
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
              deafenedBy: item.deafenedBy ?? undefined,
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

  useEffect(() => {
    if (!tokens?.accessToken) return;
    if (!sidebarQuery.trim()) return;
    if (friendsNextCursor) void loadMoreFriends();
    if (previewsNextCursor) void loadMorePreviews();
  }, [sidebarQuery, tokens, friendsNextCursor, previewsNextCursor, loadMoreFriends, loadMorePreviews]);

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
      if (err instanceof ApiError && err.code === "CHANNEL_IS_DEFAULT") {
        useChannelStore.getState().setError(tSidebar("channelIsDefault"));
        setDeleteConfirm(null);
      }
    }
  };

  const handleBlockUser = async () => {
    if (deleteConfirm?.type !== "block") return;
    try {
      await useBlockStore.getState().block(deleteConfirm.id);
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to block user:", err);
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

  const stepConversation = (delta: -1 | 1) => {
    if (selectedServer && !selectedDm) {
      const list = (channelsByServer[selectedServer.id] ?? []).filter(
        (c) => c.kind === "text",
      );
      if (list.length === 0) return;
      const idx = list.findIndex((c) => c.id === selectedChannelId);
      const next =
        list[
          idx < 0
            ? delta > 0
              ? 0
              : list.length - 1
            : (idx + delta + list.length) % list.length
        ];
      if (next.id !== selectedChannelId) handleSelectChannel(selectedServer, next);
      return;
    }
    if (acceptedFriends.length === 0) return;
    const idx = acceptedFriends.findIndex((d) => d.id === selectedDm?.id);
    const next =
      acceptedFriends[
        idx < 0
          ? delta > 0
            ? 0
            : acceptedFriends.length - 1
          : (idx + delta + acceptedFriends.length) % acceptedFriends.length
      ];
    if (next.id !== selectedDm?.id) handleSelectDm(next);
  };
  const navigationBlocked =
    settingsOpen ||
    !!serverSettings ||
    !!passwordPrompt ||
    !!deleteConfirm ||
    !!channelModal ||
    !!inviteModalServer ||
    isServerModalOpen ||
    isAddFriendOpen ||
    isPendingOpen;
  useHotkey("nextChannel", () => stepConversation(1), !navigationBlocked);
  useHotkey("prevChannel", () => stepConversation(-1), !navigationBlocked);

  const activeCallId = useCallStore((s) => s.activeCallId);
  const callFocused = useCallStore((s) => s.focused);
  const callPeerId = useCallStore((s) => s.peerId);
  const callParticipants = useCallStore((s) => s.participants);
  useEffect(() => {
    if (!callFocused || !activeCallId || !callPeerId) return;
    if (selectedDm && (selectedDm.id === callPeerId || selectedDm.id in callParticipants)) return;
    const peer = acceptedFriends.find((d) => d.id === callPeerId);
    if (!peer) return;
    setSelectedDm(peer);
    setSelectedChannel(null);
    setJoinedChannel(null);
  }, [
    callFocused,
    activeCallId,
    callPeerId,
    callParticipants,
    selectedDm,
    acceptedFriends,
    setSelectedChannel,
    setJoinedChannel,
  ]);

  const handleJoinByLink = async (token: string) => {
    const { server } = await serverAPI.joinByInvite(token);
    useServerStore.getState().addServer(server);
    setExpandedServerIds((prev) => {
      const next = new Set(prev);
      next.add(server.id);
      return next;
    });
    setSelectedServer(server.id);
    setSelectedChannel(null);
    setSelectedDm(null);
  };

  const handleCreateChannelFromModal = async (
    name: string,
    kind: ChannelKind,
    visibility: "public" | "private",
    password?: string,
  ) => {
    if (!channelModal) return;
    await createChannel(channelModal.serverId, name, kind, visibility, password);
  };

  const handlePasswordSubmit = (password: string) => {
    if (!passwordPrompt) return;
    finalizeJoin(passwordPrompt.server, passwordPrompt.channel, password);
    setPasswordPrompt(null);
  };

  const confirmDeleteAction = () => {
    if (!deleteConfirm) return;
    if (deleteConfirm.type === "server") void handleDeleteServer();
    else if (deleteConfirm.type === "leave-server") void handleLeaveServer();
    else if (deleteConfirm.type === "channel") void handleDeleteChannel();
    else if (deleteConfirm.type === "block") void handleBlockUser();
    else void handleRemoveFriend();
  };

  const handleLogout = async () => {
    if (useCallStore.getState().activeCallId) {
      await useCallStore.getState().leave();
    }
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
  const homeActive = !selectedServer && !selectedDm;
  const { fetchHome } = useHomeRefresh({
    hasSession: !!tokens?.accessToken,
    homeActive,
    watched: [voicePresenceByChannel, onlineUsers, friends],
  });

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

  const handleCallFromHome = (friendId: string, video = false) => {
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
    cs.start([friendId], { video }).catch((e) => {
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
      isDeafened?: boolean;
      deafenedBy?: string;
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
        isDeafened?: boolean;
        deafenedBy?: string;
        isSharingScreen?: boolean;
        isCameraOn?: boolean;
      }[]
    > = {};
    const joinedChannelId = joinedChannel?.channel.id ?? null;
    const selfId = user?.id ?? null;
    const liveDeafened = new Set(
      voiceParticipants.filter((p) => p.isDeafened).map((p) => p.identity),
    );
    for (const [channelId, members] of Object.entries(voicePresenceByChannel)) {
      const live = channelId === joinedChannelId;
      out[channelId] = members.map((m) => ({
        id: m.id,
        name: m.name,
        initials: initialFor(m.name),
        color: colorForId(m.id),
        isMuted: m.isMuted,
        mutedBy: m.mutedBy,
        isDeafened:
          !!m.deafenedBy ||
          (live && (m.id === selfId ? selfDeafened : liveDeafened.has(m.id))),
        deafenedBy: m.deafenedBy,
        isSharingScreen: m.screenOn,
        isCameraOn: m.cameraOn,
      }));
    }
    return out;
  }, [voicePresenceByChannel, voiceParticipants, joinedChannel?.channel.id, user?.id, selfDeafened]);


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
                    ...(selfDeafened ? { isDeafened: true } : {}),
                  },
                  ...voiceParticipants.map((p) => {
                    const name =
                      nameOverrides[p.identity] ?? (p.name || p.identity);
                    return {
                      id: p.identity,
                      name,
                      initials: initialFor(name),
                      color: colorForId(p.identity),
                      ...(p.isDeafened ? { isDeafened: true } : {}),
                    };
                  }),
                ]
              : []
          }
          voiceMembersByChannel={voiceMembersByChannel}
          selectedDmId={selectedDm?.id ?? null}
          directs={acceptedFriends}
          hasMoreDirects={!!friendsNextCursor || !!previewsNextCursor}
          onLoadMoreDirects={() => {
            void loadMoreFriends();
            void loadMorePreviews();
          }}
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
                onBlockUser={() =>
                  setDeleteConfirm({
                    type: "block",
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


      <ModalsHost
        serverModal={{
          isOpen: isServerModalOpen,
          view: serverModalView,
          token: joinToken,
          onClose: () => {
            setIsServerModalOpen(false);
            setJoinToken(null);
          },
          onCreate: handleCreateServer,
          onJoinByLink: handleJoinByLink,
        }}
        channelModal={channelModal}
        onCloseChannelModal={() => setChannelModal(null)}
        onCreateChannel={handleCreateChannelFromModal}
        passwordPrompt={passwordPrompt}
        onClosePasswordPrompt={() => setPasswordPrompt(null)}
        onSubmitPassword={handlePasswordSubmit}
        isAddFriendOpen={isAddFriendOpen}
        onCloseAddFriend={() => setIsAddFriendOpen(false)}
        isPendingOpen={isPendingOpen}
        onClosePending={() => setIsPendingOpen(false)}
        onOpenAddFriend={() => setIsAddFriendOpen(true)}
        inviteModalServer={inviteModalServer}
        acceptedFriends={friends.filter((f) => f.status === "accepted")}
        onCloseInviteModal={() => setInviteModalServer(null)}
        deleteConfirm={deleteConfirm}
        onCancelConfirm={() => setDeleteConfirm(null)}
        onConfirm={confirmDeleteAction}
        onOpenDm={handleOpenDmFromHome}
        onCall={handleCallFromHome}
        onRemoveFriend={(id, name) => setDeleteConfirm({ type: "friend", id, name })}
        onBlockUser={(id, name) => setDeleteConfirm({ type: "block", id, name })}
        callBar={{
          dockId: settingsOpen
            ? "settings-voice-dock"
            : serverSettings
              ? "server-settings-voice-dock"
              : "voice-dock-slot",
          onOpen: () => {
            setSettingsOpen(false);
            setServerSettings(null);
          },
        }}
      />
      <ErrorToast message={error} onDismiss={clearError} />
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
