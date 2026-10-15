import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { useAuthStore } from "../../auth/store/authStore";
import { communitySession, useCommunityStore } from "../store/communityStore";
import { channelsFullyLoaded, useChannelStore } from "../store/channelStore";
import { pinnedFirst, useConversationPinsStore } from "../store/conversationPinsStore";
import { useHiddenDmStore } from "../../dm/store/hiddenDmStore";
import { hydrateSidebar, readSidebarSnapshot, clearSidebarSnapshot, startSidebarSnapshots } from "../lib/sidebarSnapshot";
import { communityAPI } from "../api/community";
import { extractInviteToken, extractPackSlug, extractProfileId } from "../../../shared/lib/inviteLinks";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import { useSearchFocusStore } from "../../search/store/searchFocusStore";
import StickerPacksModal from "../../stickers/components/StickerPacksModal";
import ClearDmDialog from "../../dm/components/ClearDmDialog";
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
import { CallToast } from "../../voice/components/CallReturnBar";
import VoiceRoomLabel from "../../voice/components/VoiceRoomLabel";
import VoiceChannelPreview from "../../voice/components/VoiceChannelPreview";
import type { VoicePreviewParticipant } from "../../voice/components/VoiceChannelPreview";
import DetailsPanel from "../../chat/components/DetailsPanel";
import { useDetailsPanelStore } from "../../chat/store/detailsPanelStore";
import { usePresence } from "../../../shared/hooks/usePresence";
import { announcePanels } from "../../../shared/lib/panelWidth";
import NoTextChannel from "./NoTextChannel";
import SettingsView from "../../settings/components/SettingsView";
import { useHotkey } from "../../settings/lib/hotkeys";
import CommunitySettingsView from "./CommunitySettingsView";
import ModalsHost from "./ModalsHost";
import AppTopBar from "./AppTopBar";
import SearchDropdown from "./SearchDropdown";
import { useSidebarNavigation } from "./sidebar/useSidebarNavigation";
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
import { purgeCommunityLocally } from "../../../shared/ws/context";
import InboxMenu from "../../inbox/components/InboxMenu";
import { useDmOpenerStore } from "../../social/store/dmOpenerStore";
import { parseMessageLink } from "../../chat/lib/messageLink";
import type { OpenMessageLink } from "../../chat/lib/messageLink";
import { Channel, ChannelKind } from "../../../shared/types/channel";
import { Community } from "../../../shared/types/community";
import "../../../styles/rift.css";
import LoadingDots from "../../../shared/ui/LoadingDots";

const DETAILS_OVERLAY_QUERY = "(max-width: 900px)";
const DETAILS_ANIMATION_MS = 260;

const CommunitiesPage: React.FC = () => {
  const { t: tModals } = useTranslation("modals");
  const { t: tSidebar } = useTranslation("sidebar");
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
  useEffect(() => {
    const raw = searchParams.get("profile");
    if (!raw) return;
    setSearchParams({}, { replace: true });
    const id = extractProfileId(raw);
    if (id) useProfileModalStore.getState().open(id);
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
  const [settingsRequest, setSettingsRequest] = useState<{ id: "subscription" | "account"; seq: number } | undefined>();
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

  const sessionUserId = user?.id;
  useEffect(() => {
    if (!hasSession || !sessionUserId) return;
    const snapshot = readSidebarSnapshot();
    if (snapshot && snapshot.userId !== sessionUserId) clearSidebarSnapshot();
    return startSidebarSnapshots(sessionUserId);
  }, [hasSession, sessionUserId]);

  useEffect(() => {
    if (hasSession) {
      hydrateSidebar(useAuthStore.getState().user?.id);
      fetchCommunities();
      fetchFriends();
      useConversationPinsStore.getState().ensureLoaded();
      useHiddenDmStore.getState().ensureLoaded();
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


  const callStageShown = useCallStore((s) => !!s.activeCallId && s.focused);
  const detailsOpen = useDetailsPanelStore((s) => s.open);
  const detailsPresence = usePresence(detailsOpen, DETAILS_ANIMATION_MS);
  const detailsMounted = detailsPresence !== null;
  useEffect(() => {
    if (!detailsMounted) announcePanels("--w-details");
  }, [detailsMounted]);
  const detailsTarget: React.ComponentProps<typeof DetailsPanel>["target"] | null = selectedDm
    ? {
        kind: "dm",
        friendId: selectedDm.id,
        name: nameOverrides[selectedDm.id] ?? selectedDm.name,
        color: selectedDm.color,
        isFriend: selectedDm.isFriend !== false,
      }
    : onTextChannelActive && activeTextChannel && selectedCommunity
      ? {
          kind: "channel",
          communityId: selectedCommunity.id,
          channelId: activeTextChannel.id,
          name: activeTextChannel.name,
        }
      : null;
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
    const session = communitySession();
    const community = await createCommunity(name, description, accentColor);
    if (session !== communitySession()) return;
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
    if (session !== communitySession()) return;
    openCommunity(community.id);
    if (logoError) return { logoError };
  };

  const handleDeleteCommunity = async () => {
    if (deleteConfirm?.type !== "community") return;
    try {
      await deleteCommunity(deleteConfirm.id);
      useConversationPinsStore.getState().forget("communities", deleteConfirm.id);
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
      purgeCommunityLocally(deleteConfirm.id);
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
    const ordered = pinnedFirst(acceptedFriends, useConversationPinsStore.getState().dms);
    if (ordered.length === 0) return;
    const idx = ordered.findIndex((d) => d.id === selectedDm?.id);
    const next =
      ordered[
        idx < 0
          ? delta > 0
            ? 0
            : ordered.length - 1
          : (idx + delta + ordered.length) % ordered.length
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
    const session = communitySession();
    const { community } = await communityAPI.joinByInvite(token);
    if (session !== communitySession()) return;
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
  const sidebarNav = useSidebarNavigation({
    selectedChannelId,
    selectedDmId: selectedDm?.id ?? null,
    channelsByCommunity,
  });
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

  const handleOpenDmFromHome = (friendId: string, name?: string, knownFriend = false): boolean => {
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
      isFriend: knownFriend,
      friendship: knownFriend ? "accepted" : "none",
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
    useCallStore.getState().callFriend(friendId, video);
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
      <AppTopBar
        inbox={<InboxMenu onOpenCommunity={selection.handleGoToCommunity} onOpenFriendRequests={() => setIsPendingOpen(true)} />}
        navState={sidebarNav.navState}
        goHistory={sidebarNav.goHistory}
        homeActive={homeActive}
        onGoHome={selection.handleGoHome}
        query={sidebarQuery}
        onQueryChange={setSidebarQuery}
        currentUserId={user?.id}
        currentUserName={userName}
        currentUserStatus={userStatus}
        onOpenSettings={() => setSettingsOpen(true)}
        onSetStatus={handleSetStatus}
        onSignOut={() => void handleLogout()}
        renderResults={(done) => (
          <SearchDropdown
            query={sidebarQuery}
            currentUserId={user?.id}
            directs={acceptedFriends}
            communities={communities}
            channelsByCommunity={channelsByCommunity}
            onPickDm={handleSelectDm}
            onPickCommunity={(community) => {
              if (!expandedCommunityIds.has(community.id)) selection.handleToggleCommunity(community.id);
              sidebarNav.setOpenCommunityId(community.id);
              selection.handleSelectCommunity(community.id);
            }}
            onPickChannel={(community, channel) => {
              handleSelectChannel(community, channel);
              if (!expandedCommunityIds.has(community.id)) selection.handleToggleCommunity(community.id);
              sidebarNav.setOpenCommunityId(community.id);
            }}
            onOpenResult={setPendingMessageLink}
            onMessagePerson={(userId, name, friend) => {
              handleOpenDmFromHome(userId, name, friend);
            }}
            onSearch={setSidebarQuery}
            isFriend={(userId) => useFriendshipStore.getState().statusById[userId] === "accepted"}
            onJoinInvite={handleJoinByLink}
            onDone={done}
          />
        )}
      />
      <div className="body-area">
        <Sidebar
          nav={sidebarNav}
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
          onAddFriend={() => useSearchFocusStore.getState().focus("people")}
          onCreateCommunity={() => {
            setCommunityModalView("create");
            setIsCommunityModalOpen(true);
          }}
          onJoinCommunity={() => useSearchFocusStore.getState().focus("invite")}
          currentUserId={user?.id}
          unreadByChannel={unreadByChannel}
          unreadByDm={unreadByDm}
          channelToCommunity={channelToCommunity}
        />

        <main className="main">
          <div className="main-inner">
            <CallToast onOpenDm={handleOpenDmFromHome} />
            {activeCallId && callFocused && !selectedDm ? (
              <CallStage
                onMinimize={() => useCallStore.getState().setFocused(false)}
              />
            ) : !selectedCommunity && !selectedDm ? (
              <HomeView
                overview={homeOverview}
                overviewFailed={homeFailed}
                onRetry={() => void fetchHome()}
                friends={friends}
                onlineUsers={onlineUsers}
                joinedChannelId={joinedChannel?.channel.id ?? null}
                onJoinRoom={(communityId, channelId) => void handleJoinFromHome(communityId, channelId)}
                onOpenDm={handleOpenDmFromHome}
                onCall={handleCallFromHome}
                onAddFriend={() => useSearchFocusStore.getState().focus("people")}
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
                <VoiceRoomLabel
                  overlay
                  communityName={communities.find((c) => c.id === joinedChannel.communityId)?.name}
                  channelName={joinedChannel.channel.name}
                />
                <div id="voice-stage-slot" className="voice-stage-slot" />
              </div>
            ) : !selectedCommunityChannelsLoaded ? (
              <div className="empty-state" style={{ padding: 60 }}>
                <LoadingDots />
              </div>
            ) : selectedVoiceChannelNotJoined && selectedCommunity ? (
              <VoiceChannelPreview
                communityName={selectedCommunity.name}
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

          {detailsPresence && detailsTarget && (
            <>
              {detailsPresence !== "exit" && (
                <PanelResizer
                  cssVar="--w-details"
                  otherSelector=".sb"
                  side="end"
                  min={280}
                  max={480}
                  fallback={340}
                  storageKey="rift.panel.details"
                  label={i18n.t("details.resize", { ns: "chat" })}
                />
              )}
              <div className={"details-slot " + detailsPresence}>
                <DetailsPanel
                  target={detailsTarget}
                  currentUserId={user?.id}
                  onShowMessage={(messageId) => {
                    const convId = detailsTarget.kind === "dm" ? detailsTarget.friendId : detailsTarget.channelId;
                    if (window.matchMedia?.(DETAILS_OVERLAY_QUERY).matches) useDetailsPanelStore.getState().setOpen(false);
                    setOpenMessage({ convId, messageId });
                  }}
                />
              </div>
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
      <div id="voice-dock-slot" className="call-dock" />

      {linkedPack && <StickerPacksModal packId={linkedPack} onClose={() => setLinkedPack(null)} />}
      <ClearDmDialog />

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
        isPendingOpen={isPendingOpen}
        onClosePending={() => setIsPendingOpen(false)}
        onOpenAddFriend={() => useSearchFocusStore.getState().focus("people")}
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
        onEditProfile={() => {
          setCommunitySettings(null);
          setSettingsRequest((prev) => ({ id: "account", seq: (prev?.seq ?? 0) + 1 }));
          setSettingsOpen(true);
        }}
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
