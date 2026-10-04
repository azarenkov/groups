import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { useAuthStore } from "../../auth/store/authStore";
import { useCommunityStore } from "../store/communityStore";
import { channelsFullyLoaded, useChannelStore } from "../store/channelStore";
import { communityAPI } from "../api/community";
import { extractInviteToken, extractPackSlug } from "../../../shared/lib/inviteLinks";
import StickerPacksModal from "../../stickers/components/StickerPacksModal";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { useBlockStore } from "../../social/store/blockStore";
import {
  useStreamViewersStore,
  streamViewerKey,
} from "../../voice/store/streamViewersStore";
import { useUnreadStore } from "../../chat/store/unreadStore";
import { useCallStore } from "../../voice/store/callStore";
import { useHomeStore } from "../../home/store/homeStore";
import HomeView from "../../home/components/HomeView";
import { colorForId, initialFor } from "../../../shared/lib/avatar";
import { bustAvatar } from "../../../shared/lib/avatarCache";
import { ApiError } from "../../../shared/lib/apiError";
import i18n from "../../../i18n";
import Sidebar from "./Sidebar";
import ChatView from "../../chat/components/ChatView";
import VoiceView from "../../voice/components/VoiceView";
import CallStage from "../../voice/components/CallStage";
import CallReturnBar, { CallErrorToast } from "../../voice/components/CallReturnBar";
import VoiceChannelPreview from "../../voice/components/VoiceChannelPreview";
import type { VoicePreviewParticipant } from "../../voice/components/VoiceChannelPreview";
import MembersPanel from "./MembersPanel";
import SharedPanel from "../../chat/components/SharedPanel";
import NoTextChannel from "./NoTextChannel";
import SettingsView from "../../settings/components/SettingsView";
import { useHotkey } from "../../settings/lib/hotkeys";
import CommunitySettingsView from "./CommunitySettingsView";
import ModalsHost from "./ModalsHost";
import { useVoiceSession } from "../../voice/hooks/useVoiceSession";
import { useHomeRefresh } from "../../home/hooks/useHomeRefresh";
import { useActiveChatView } from "../hooks/useActiveChatView";
import { useAppearOffline } from "../hooks/useAppearOffline";
import { useDirectConversations } from "../hooks/useDirectConversations";
import { useSelection } from "../hooks/useSelection";
import { useSidebarPrefetch } from "../hooks/useSidebarPrefetch";
import { useVoiceMembers } from "../hooks/useVoiceMembers";
import { useVoicePresenceSync } from "../hooks/useVoicePresenceSync";
import { useSpeakingTracking } from "../../voice/hooks/useSpeakingTracking";
import { useEntitlementsStore } from "../../subscription/store/entitlementsStore";
import { useStreamQualitySync } from "../../subscription/hooks/useStreamQualitySync";
import ErrorToast from "../../../shared/ui/ErrorToast";
import TrialEndingNotice from "../../subscription/components/TrialEndingNotice";
import PanelResizer from "../../../shared/ui/PanelResizer";
import ContextMenuHost from "../../../shared/ui/contextMenu/ContextMenuHost";
import { useDmOpenerStore } from "../../social/store/dmOpenerStore";
import { parseMessageLink } from "../../chat/lib/messageLink";
import type { OpenMessageLink } from "../../chat/lib/messageLink";
import Icon from "../../../shared/ui/Icon";
import { useLocalQuality } from "../../voice/hooks/useLocalQuality";
import { qualityClass, qualityLabel } from "../../voice/lib/quality";
import { Channel, ChannelKind } from "../../../shared/types/channel";
import { Community } from "../../../shared/types/community";
import "../../../styles/rift.css";

const SHARED_OVERLAY_QUERY = "(max-width: 900px)";

const CommunitiesPage: React.FC = () => {
  const { t: tModals } = useTranslation("modals");
  const { t: tVoice } = useTranslation("voice");
  const { t: tSidebar } = useTranslation("sidebar");
  const localQuality = useLocalQuality();
  const streamViewerCounts = useStreamViewersStore((s) => s.counts);
  const unreadByChannel = useUnreadStore((s) => s.byChannel);
  const unreadByDm = useUnreadStore((s) => s.byDm);
  const channelToCommunity = useUnreadStore((s) => s.channelToCommunity);
  const { tokens, user, signOut } = useAuthStore();
  const hasSession = !!tokens?.accessToken;
  const {
    communities,
    selectedCommunityId,
    error: communitiesError,
    setSelectedCommunity,
    fetchCommunities,
    createCommunity,
    deleteCommunity,
    clearError: clearCommunitiesError,
  } = useCommunityStore();

  const {
    channelsByCommunity,
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

  const [isCommunityModalOpen, setIsCommunityModalOpen] = useState(false);
  const [communityModalView, setCommunityModalView] = useState<
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
    setCommunityModalView("join");
    setIsCommunityModalOpen(true);
  }, [searchParams, setSearchParams]);
  const [linkedPack, setLinkedPack] = useState<string | null>(null);
  useEffect(() => {
    const raw = searchParams.get("stickers");
    if (!raw) return;
    setSearchParams({}, { replace: true });
    setLinkedPack(extractPackSlug(raw));
  }, [searchParams, setSearchParams]);
  const [pendingMessageLink, setPendingMessageLink] = useState<OpenMessageLink | null>(null);
  const linkRefreshRef = React.useRef<{ link: OpenMessageLink | null; communities: boolean; channels: boolean }>({
    link: null,
    communities: false,
    channels: false,
  });
  const [openMessage, setOpenMessage] = useState<{ convId: string; messageId: string } | null>(null);
  useEffect(() => {
    const link = parseMessageLink(searchParams, user?.id);
    if (!link) return;
    setSearchParams({}, { replace: true });
    setPendingMessageLink(link);
  }, [searchParams, setSearchParams, user?.id]);
  const [isAddFriendOpen, setIsAddFriendOpen] = useState(false);
  const [isPendingOpen, setIsPendingOpen] = useState(false);
  const [inviteModalCommunity, setInviteModalCommunity] = useState<Community | null>(null);
  const [communitySettings, setCommunitySettings] = useState<Community | null>(null);
  const [settingsChannels, setSettingsChannels] = useState<{
    communityId: string;
    state: "loading" | "error" | "ready";
  } | null>(null);
  const loadSettingsChannels = (communityId: string) => {
    setSettingsChannels({ communityId, state: "loading" });
    const finish = (state: "error" | "ready") =>
      setSettingsChannels((prev) => (prev?.communityId === communityId ? { communityId, state } : prev));
    fetchChannels(communityId).then(
      (ok) => finish(ok ? "ready" : "error"),
      () => finish("error"),
    );
  };
  const [channelModal, setChannelModal] = useState<{ communityId: string; kind: ChannelKind } | null>(null);
  const fetchFriends = useFriendshipStore((s) => s.fetchFriends);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsRequest, setSettingsRequest] = useState<{ id: "subscription"; seq: number } | undefined>();
  useEffect(() => {
    if (!settingsOpen) setSettingsRequest(undefined);
  }, [settingsOpen]);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    type: "community" | "leave-community" | "channel" | "friend" | "block";
    id: string;
    communityId?: string;
    name: string;
  } | null>(null);
  const [sidebarQuery, setSidebarQuery] = useState("");
  const [membersOpenById, setMembersOpenById] = useState<Record<string, boolean>>({});
  const [sharedOpenFor, setSharedOpenFor] = useState<string | null>(null);


  const voice = useVoiceSession();
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
    reportStreamWatch,
  } = voice;

  const selection = useSelection({
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
  });
  const {
    selectedDm,
    setSelectedDm,
    expandedCommunityIds,
    collapseCommunity,
    selectedCommunity,
    activeTextChannel,
    selectedVoiceChannelNotJoined,
    selectedCommunityChannelsLoaded,
    communityHasTextChannel,
    onTextChannelActive,
    handleSelectChannel,
    handleSelectDm,
    finalizeJoin,
    openCommunity,
  } = selection;

  const {
    friends,
    acceptedFriends,
    pendingCount,
    onlineUsers,
    nameOverrides,
    dmPreviews,
  } = useDirectConversations({
    currentUserId: user?.id,
    selectedDm,
    setSelectedDm,
  });

  const userName = user?.name || "You";
  const { voiceMembers, voiceMembersByChannel, voicePresenceByChannel } =
    useVoiceMembers({
      joinedChannelId: joinedChannel?.channel.id ?? null,
      currentUserId: user?.id,
      currentUserName: userName,
      nameOverrides,
    });
  const { userStatus, setStatus: handleSetStatus } = useAppearOffline({
    hasSession,
    currentUserId: user?.id,
  });

  useEffect(() => {
    if (hasSession) {
      fetchCommunities();
      fetchFriends();
      void useBlockStore.getState().fetch();
      void useEntitlementsStore.getState().fetch();
    }
  }, [hasSession, fetchCommunities, fetchFriends]);

  useVoicePresenceSync({ communities, hasSession });
  useSpeakingTracking();
  useStreamQualitySync();
  const { hasMoreDirects, loadMoreDirects } = useSidebarPrefetch({
    query: sidebarQuery,
    hasSession,
    communities,
    channelsByCommunity,
    fetchChannels,
  });

  const previewParticipants: VoicePreviewParticipant[] = React.useMemo(() => {
    if (!selectedVoiceChannelNotJoined) return [];
    const members =
      voicePresenceByChannel[selectedVoiceChannelNotJoined.id] || [];
    return members.map((m) => ({
      id: m.id,
      name: m.name,
      isMuted: m.isMuted,
      cameraOn: m.cameraOn,
      screenOn: m.screenOn,
    }));
  }, [selectedVoiceChannelNotJoined, voicePresenceByChannel]);

  const channelSharedKey = activeTextChannel ? `ch:${activeTextChannel.id}` : null;
  const dmSharedKey = selectedDm ? `dm:${selectedDm.id}` : null;
  const showSharedForChannel =
    onTextChannelActive && !!activeTextChannel && !!selectedCommunity && sharedOpenFor === channelSharedKey;
  const showSharedForDm = !!selectedDm && sharedOpenFor === dmSharedKey;
  const showMembersForChannel =
    onTextChannelActive && !!activeTextChannel && !!membersOpenById[activeTextChannel.id] && !showSharedForChannel;

  const callStageShown = useCallStore((s) => !!s.activeCallId && s.focused);
  useActiveChatView({
    textChannelId:
      onTextChannelActive && activeTextChannel ? activeTextChannel.id : null,
    dmId: selectedDm?.id ?? null,
    blocked: settingsOpen || !!communitySettings || callStageShown,
  });

  const handleCreateCommunity = async (
    name: string,
    description?: string,
    accentColor?: string,
    logoFile?: File,
  ): Promise<void | { logoError?: string }> => {
    const community = await createCommunity(name, description, accentColor);
    let logoError: string | undefined;
    if (logoFile) {
      try {
        await communityAPI.uploadLogo(community.id, logoFile);
        bustAvatar("community", community.id);
      } catch (err) {
        console.error("[CommunitiesPage] failed to upload community logo:", err);
        const apiErr = err instanceof ApiError ? err : null;
        const reason =
          apiErr?.code === "IMAGE_CONTENT_REJECTED"
            ? tModals("communityModal.logoRejected")
            : apiErr?.message || tModals("communityModal.logoUploadGeneric");
        logoError = tModals("communityModal.logoUploadFailed", { reason });
      }
    }
    try {
      await createChannel(community.id, "voice", "voice", "public");
    } catch (err) {
      console.error("[CommunitiesPage] failed to create default voice channel:", err);
    }
    openCommunity(community.id);
    if (logoError) return { logoError };
  };

  const handleDeleteCommunity = async () => {
    if (deleteConfirm?.type !== "community") return;
    try {
      await deleteCommunity(deleteConfirm.id);
      if (selectedCommunityId === deleteConfirm.id) setSelectedCommunity(null);
      clearChannels(deleteConfirm.id);
      if (joinedChannel?.communityId === deleteConfirm.id) {
        setJoinedChannel(null);
        setVoiceJoinFailed(false);
      }
      collapseCommunity(deleteConfirm.id);
      setCommunitySettings((prev) => (prev?.id === deleteConfirm.id ? null : prev));
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to delete community:", err);
    }
  };

  const handleLeaveCommunity = async () => {
    if (deleteConfirm?.type !== "leave-community") return;
    try {
      await communityAPI.leave(deleteConfirm.id);
      if (selectedCommunityId === deleteConfirm.id) setSelectedCommunity(null);
      clearChannels(deleteConfirm.id);
      if (joinedChannel?.communityId === deleteConfirm.id) {
        setJoinedChannel(null);
        setVoiceJoinFailed(false);
      }
      collapseCommunity(deleteConfirm.id);
      await fetchCommunities();
      setCommunitySettings((prev) => (prev?.id === deleteConfirm.id ? null : prev));
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to leave community:", err);
    }
  };

  const handleDeleteChannel = async () => {
    if (deleteConfirm?.type !== "channel" || !deleteConfirm.communityId) return;
    try {
      await deleteChannel(deleteConfirm.communityId, deleteConfirm.id);
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
    if (selectedCommunity && !selectedDm) {
      const list = (channelsByCommunity[selectedCommunity.id] ?? []).filter(
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
      if (next.id !== selectedChannelId) handleSelectChannel(selectedCommunity, next);
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
    !!communitySettings ||
    !!passwordPrompt ||
    !!deleteConfirm ||
    !!channelModal ||
    !!inviteModalCommunity ||
    isCommunityModalOpen ||
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
    setSelectedDm,
    setSelectedChannel,
    setJoinedChannel,
  ]);

  const handleJoinByLink = async (token: string) => {
    const { community } = await communityAPI.joinByInvite(token);
    useCommunityStore.getState().addCommunity(community);
    openCommunity(community.id);
  };

  const handleCreateChannelFromModal = async (
    name: string,
    kind: ChannelKind,
    visibility: "public" | "private",
    password?: string,
  ) => {
    if (!channelModal) return;
    await createChannel(channelModal.communityId, name, kind, visibility, password);
  };

  const handlePasswordSubmit = (password: string) => {
    if (!passwordPrompt) return;
    finalizeJoin(passwordPrompt.community, passwordPrompt.channel, password);
    setPasswordPrompt(null);
  };

  const confirmDeleteAction = () => {
    if (!deleteConfirm) return;
    if (deleteConfirm.type === "community") void handleDeleteCommunity();
    else if (deleteConfirm.type === "leave-community") void handleLeaveCommunity();
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

  const error = communitiesError || channelsError;
  const clearError = () => {
    clearCommunitiesError();
    clearChannelsError();
  };

  const homeOverview = useHomeStore((s) => s.overview);
  const homeFailed = useHomeStore((s) => s.failed);
  const homeActive = !selectedCommunity && !selectedDm;
  const { fetchHome } = useHomeRefresh({
    hasSession,
    homeActive,
    watched: [voicePresenceByChannel, onlineUsers, friends],
  });

  const handleJoinFromHome = async (communityId: string, channelId: string) => {
      const community = communities.find((s) => s.id === communityId);
      if (!community) return;
      let channel = channelsByCommunity[communityId]?.find((c) => c.id === channelId);
      if (!channel) {
        await fetchChannels(communityId);
        channel = useChannelStore
          .getState()
          .channelsByCommunity[communityId]?.find((c) => c.id === channelId);
      }
      if (!channel) return;
      selection.expandCommunity(communityId);
      handleSelectChannel(community, channel);
  };

  const handleOpenDmFromHome = (friendId: string, name?: string): boolean => {
    const dm = acceptedFriends.find((d) => d.id === friendId);
    if (dm) {
      handleSelectDm(dm);
      return true;
    }
    const preview = dmPreviews[friendId];
    const fallbackName = name ?? nameOverrides[friendId] ?? preview?.peerName;
    if (!fallbackName) return false;
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
    return true;
  };
  const openDmRef = React.useRef(handleOpenDmFromHome);
  openDmRef.current = handleOpenDmFromHome;
  const expandCommunityRef = React.useRef(selection.expandCommunity);
  expandCommunityRef.current = selection.expandCommunity;
  useEffect(() => {
    useDmOpenerStore.getState().register((id, name) => openDmRef.current(id, name));
    return () => useDmOpenerStore.getState().register(null);
  }, []);
  useEffect(() => {
    const link = pendingMessageLink;
    if (!link) return;
    if (link.kind === "dm") {
      if (!openDmRef.current(link.peerId, link.peerName)) return;
      setPendingMessageLink(null);
      setOpenMessage({ convId: link.peerId, messageId: link.messageId });
      return;
    }
    const refreshed = linkRefreshRef.current.link === link ? linkRefreshRef.current : { link, communities: false, channels: false };
    linkRefreshRef.current = refreshed;
    const drop = () => setPendingMessageLink((current) => (current === link ? null : current));
    if (!communities.some((c) => c.id === link.communityId)) {
      if (refreshed.communities) return;
      refreshed.communities = true;
      void fetchCommunities().then(() => {
        if (!useCommunityStore.getState().communities.some((c) => c.id === link.communityId)) drop();
      });
      return;
    }
    if (!channelsByCommunity[link.communityId]?.some((c) => c.id === link.channelId)) {
      if (refreshed.channels) return;
      refreshed.channels = true;
      void fetchChannels(link.communityId).then((ok) => {
        const found = useChannelStore
          .getState()
          .channelsByCommunity[link.communityId]?.some((c) => c.id === link.channelId);
        if (!ok || (!found && channelsFullyLoaded(link.communityId))) drop();
      });
      return;
    }
    setPendingMessageLink(null);
    expandCommunityRef.current(link.communityId);
    useCallStore.getState().setFocused(false);
    setSelectedDm(null);
    setSelectedCommunity(link.communityId);
    setSelectedChannel(link.channelId);
    setOpenMessage({ convId: link.channelId, messageId: link.messageId });
  }, [
    pendingMessageLink,
    communities,
    channelsByCommunity,
    fetchChannels,
    fetchCommunities,
    acceptedFriends,
    dmPreviews,
    setSelectedDm,
    setSelectedCommunity,
    setSelectedChannel,
  ]);

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

  const connectToPreviewChannel = (community: Community, channel: Channel) => {
    if (channel.visibility === "private") {
      setPasswordPrompt({ community, channel });
      return;
    }
    finalizeJoin(community, channel);
  };

  return (
    <div className="win">
      <div className="body-area">
        <Sidebar
          communities={communities}
          channelsByCommunity={channelsByCommunity}
          expandedCommunityIds={expandedCommunityIds}
          selectedChannelId={selectedChannelId}
          joinedChannelId={joinedChannel?.channel.id ?? null}
          voiceMembers={voiceMembers}
          voiceMembersByChannel={voiceMembersByChannel}
          selectedDmId={selectedDm?.id ?? null}
          directs={acceptedFriends}
          hasMoreDirects={hasMoreDirects}
          onLoadMoreDirects={loadMoreDirects}
          onToggleCommunity={selection.handleToggleCommunity}
          onSelectCommunity={selection.handleSelectCommunity}
          onSelectChannel={handleSelectChannel}
          onSelectDm={handleSelectDm}
          onInviteCommunity={(community) => setInviteModalCommunity(community)}
          onOpenCommunitySettings={(community) => {
            setCommunitySettings(community);
            if (channelsFullyLoaded(community.id)) setSettingsChannels(null);
            else loadSettingsChannels(community.id);
          }}
          onLeaveCommunity={(community) =>
            setDeleteConfirm({
              type: community.ownerId === user?.id ? "community" : "leave-community",
              id: community.id,
              name: community.name,
            })
          }
          onCallDm={(dm) => handleCallFromHome(dm.id)}
          onAddChannel={(community, kind) => setChannelModal({ communityId: community.id, kind })}
          onDeleteChannel={(community, channel) =>
            setDeleteConfirm({
              type: "channel",
              id: channel.id,
              communityId: community.id,
              name: channel.name,
            })
          }
          onOpenSettings={() => setSettingsOpen(true)}
          onAddFriend={() => setIsAddFriendOpen(true)}
          onOpenPending={() => setIsPendingOpen(true)}
          onGoHome={selection.handleGoHome}
          homeActive={homeActive}
          pendingCount={pendingCount}
          onCreateCommunity={() => {
            setCommunityModalView("create");
            setIsCommunityModalOpen(true);
          }}
          onJoinCommunity={() => {
            setCommunityModalView("join");
            setIsCommunityModalOpen(true);
          }}
          currentUserId={user?.id}
          currentUserName={userName}
          currentUserStatus={userStatus}
          searchQuery={sidebarQuery}
          onSearchQueryChange={setSidebarQuery}
          onOpenSearchResult={setPendingMessageLink}
          unreadByChannel={unreadByChannel}
          unreadByDm={unreadByDm}
          channelToCommunity={channelToCommunity}
          onSetStatus={handleSetStatus}
        />

        <PanelResizer
          cssVar="--w-sidebar"
          otherSelector=".members-panel"
          side="start"
          min={240}
          max={420}
          fallback={320}
          storageKey="rift.panel.sidebar"
          label={i18n.t("resizeSidebar", { ns: "common" })}
        />
        <main className="main">
          <div className="main-inner">
            <CallReturnBar />
            <CallErrorToast />
            {activeCallId && callFocused && !selectedDm ? (
              <CallStage
                onMinimize={() => useCallStore.getState().setFocused(false)}
              />
            ) : !selectedCommunity && !selectedDm ? (
              <HomeView
                userName={userName}
                overview={homeOverview}
                overviewFailed={homeFailed}
                onRetry={() => void fetchHome()}
                friends={friends}
                onlineUsers={onlineUsers}
                joinedChannelId={joinedChannel?.channel.id ?? null}
                onJoinRoom={(communityId, channelId) => void handleJoinFromHome(communityId, channelId)}
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
                openMessageId={openMessage?.convId === selectedDm.id ? openMessage.messageId : undefined}
                onOpenMessageHandled={() => setOpenMessage(null)}
                sharedOpen={showSharedForDm}
                onToggleShared={() => setSharedOpenFor((open) => (open === dmSharedKey ? null : dmSharedKey))}
                communities={communities}
                onGoToCommunity={selection.handleGoToCommunity}
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
                    {selectedCommunity ? ` · ${selectedCommunity.name}` : ""}
                  </span>
                </header>
                <div id="voice-stage-slot" className="voice-stage-slot" />
              </div>
            ) : !selectedCommunityChannelsLoaded ? (
              <div className="empty-state" style={{ padding: 60 }}>
                <span className="loading-spinner" />
              </div>
            ) : selectedVoiceChannelNotJoined && selectedCommunity ? (
              <VoiceChannelPreview
                channelId={selectedVoiceChannelNotJoined.id}
                channelName={selectedVoiceChannelNotJoined.name}
                participants={previewParticipants}
                onConnect={() =>
                  connectToPreviewChannel(
                    selectedCommunity,
                    selectedVoiceChannelNotJoined,
                  )
                }
                onInvite={() => setInviteModalCommunity(selectedCommunity)}
              />
            ) : !communityHasTextChannel ? (
              <NoTextChannel
                communityName={selectedCommunity?.name ?? ""}
                onCreate={() => {
                  if (selectedCommunity) {
                    setChannelModal({ communityId: selectedCommunity.id, kind: "text" });
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
                communityId={selectedCommunity?.id}
                currentUserId={user?.id}
                openMessageId={
                  activeTextChannel && openMessage?.convId === activeTextChannel.id
                    ? openMessage.messageId
                    : undefined
                }
                onOpenMessageHandled={() => setOpenMessage(null)}
                membersOpen={
                  !!activeTextChannel ? !!membersOpenById[activeTextChannel.id] : false
                }
                onToggleMembers={
                  activeTextChannel
                    ? () => {
                        setMembersOpenById((prev) => ({
                          ...prev,
                          [activeTextChannel.id]: showSharedForChannel ? true : !prev[activeTextChannel.id],
                        }));
                        if (showSharedForChannel) setSharedOpenFor(null);
                      }
                    : undefined
                }
                sharedOpen={showSharedForChannel}
                onToggleShared={
                  activeTextChannel && selectedCommunity
                    ? () => setSharedOpenFor((open) => (open === channelSharedKey ? null : channelSharedKey))
                    : undefined
                }
              />
            )}

            {joinedChannel && (
              <VoiceView
                key={`${joinedChannel.channel.id}-${voiceJoinAttempt}`}
                channelId={joinedChannel.channel.id}
                communityId={joinedChannel.communityId}
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
                      joinedChannel.communityId,
                      joinedChannel.channel.id,
                      id === (user?.id || "you") ? user?.id || "you" : id,
                    )
                  ]
                }
                onStreamWatch={reportStreamWatch}
                onOpenStage={() => {
                  useCallStore.getState().setFocused(false);
                  setSelectedDm(null);
                  setSelectedCommunity(joinedChannel.communityId);
                  setSelectedChannel(joinedChannel.channel.id);
                }}
                onLeave={handleVoiceLeave}
                onJoinFailed={handleVoiceJoinFailed}
              />
            )}
          </div>

          {(showSharedForDm || showSharedForChannel) && (
            <>
              <PanelResizer
                cssVar="--w-shared"
                otherSelector=".sb"
                side="end"
                min={280}
                max={480}
                fallback={340}
                storageKey="rift.panel.shared"
                label={i18n.t("shared.resize", { ns: "chat" })}
              />
              <SharedPanel
                key={showSharedForDm ? dmSharedKey ?? "" : channelSharedKey ?? ""}
                scope={
                  showSharedForDm && selectedDm
                    ? { kind: "dm", friendId: selectedDm.id }
                    : { kind: "channel", communityId: selectedCommunity!.id, channelId: activeTextChannel!.id }
                }
                currentUserId={user?.id}
                onClose={() => setSharedOpenFor(null)}
                onShowMessage={(messageId) => {
                  const convId = showSharedForDm && selectedDm ? selectedDm.id : activeTextChannel?.id;
                  if (!convId) return;
                  if (window.matchMedia?.(SHARED_OVERLAY_QUERY).matches) setSharedOpenFor(null);
                  setOpenMessage({ convId, messageId });
                }}
              />
            </>
          )}
          {showMembersForChannel && selectedCommunity && activeTextChannel && (
            <>
              <PanelResizer
                cssVar="--w-members"
                otherSelector=".sb"
                side="end"
                min={220}
                max={360}
                fallback={240}
                storageKey="rift.panel.members"
                label={i18n.t("resizeMembers", { ns: "common" })}
              />
              <MembersPanel
                communityId={selectedCommunity.id}
                channelId={activeTextChannel.id}
              />
            </>
          )}
        </main>

        {settingsOpen && (
          <SettingsView
            open={settingsOpen}
            onClose={() => setSettingsOpen(false)}
            onSignOut={handleLogout}
            sectionRequest={settingsRequest}
          />
        )}

        {communitySettings && (
          <CommunitySettingsView
            community={
              communities.find((s) => s.id === communitySettings.id) ?? communitySettings
            }
            friends={friends}
            channels={channelsByCommunity[communitySettings.id] ?? []}
            channelsState={
              settingsChannels?.communityId === communitySettings.id
                ? settingsChannels.state
                : channelsFullyLoaded(communitySettings.id)
                  ? "ready"
                  : "loading"
            }
            onRetryChannels={() => loadSettingsChannels(communitySettings.id)}
            currentUserId={user?.id}
            onClose={() => {
              setCommunitySettings(null);
              setSettingsChannels(null);
            }}
            onDeleteCommunity={(community) => {
              const isOwner = community.ownerId === user?.id;
              setDeleteConfirm({
                type: isOwner ? "community" : "leave-community",
                id: community.id,
                name: community.name,
              });
            }}
            onCreateChannel={(name, kind, visibility, password) =>
              createChannel(communitySettings.id, name, kind, visibility, password)
            }
            onDeleteChannel={(channel) =>
              setDeleteConfirm({
                type: "channel",
                id: channel.id,
                communityId: communitySettings.id,
                name: channel.name,
              })
            }
          />
        )}
      </div>

      {linkedPack && <StickerPacksModal packId={linkedPack} onClose={() => setLinkedPack(null)} />}

      <ModalsHost
        communityModal={{
          isOpen: isCommunityModalOpen,
          view: communityModalView,
          token: joinToken,
          onClose: () => {
            setIsCommunityModalOpen(false);
            setJoinToken(null);
          },
          onCreate: handleCreateCommunity,
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
        inviteModalCommunity={inviteModalCommunity}
        acceptedFriends={friends.filter((f) => f.status === "accepted")}
        onCloseInviteModal={() => setInviteModalCommunity(null)}
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
            : communitySettings
              ? "community-settings-voice-dock"
              : "voice-dock-slot",
          onOpen: () => {
            setSettingsOpen(false);
            setCommunitySettings(null);
          },
        }}
      />
      <ErrorToast message={error} onDismiss={clearError} />
      <TrialEndingNotice
        onOpenSubscription={() => {
          setCommunitySettings(null);
          setSettingsRequest((prev) => ({ id: "subscription", seq: (prev?.seq ?? 0) + 1 }));
          setSettingsOpen(true);
        }}
      />
      <ContextMenuHost />
    </div>
  );
};

export default CommunitiesPage;
