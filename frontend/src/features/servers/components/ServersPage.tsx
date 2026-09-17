import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { useAuthStore } from "../../auth/store/authStore";
import { useServerStore } from "../store/serverStore";
import { useChannelStore } from "../store/channelStore";
import { serverAPI } from "../api/server";
import { extractInviteToken } from "../../../shared/lib/inviteLinks";
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
import MembersPanel from "./MembersPanel";
import NoTextChannel from "./NoTextChannel";
import SettingsView from "../../settings/components/SettingsView";
import { useHotkey } from "../../settings/lib/hotkeys";
import ServerSettingsView from "./ServerSettingsView";
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
import { useEntitlementsStore } from "../../subscription/store/entitlementsStore";
import { useStreamQualitySync } from "../../subscription/hooks/useStreamQualitySync";
import ErrorToast from "../../../shared/ui/ErrorToast";
import Icon from "../../../shared/ui/Icon";
import { useLocalQuality } from "../../voice/hooks/useLocalQuality";
import { qualityClass, qualityLabel } from "../../voice/lib/quality";
import { Channel, ChannelKind, ChannelParticipant } from "../../../shared/types/channel";
import { Server } from "../../../shared/types/server";
import "../../../styles/rift.css";

const ServersPage: React.FC = () => {
  const { t: tModals } = useTranslation("modals");
  const { t: tVoice } = useTranslation("voice");
  const { t: tSidebar } = useTranslation("sidebar");
  const localQuality = useLocalQuality();
  const streamViewerCounts = useStreamViewersStore((s) => s.counts);
  const unreadByChannel = useUnreadStore((s) => s.byChannel);
  const unreadByDm = useUnreadStore((s) => s.byDm);
  const channelToServer = useUnreadStore((s) => s.channelToServer);
  const { tokens, user, signOut } = useAuthStore();
  const hasSession = !!tokens?.accessToken;
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
  const fetchFriends = useFriendshipStore((s) => s.fetchFriends);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    type: "server" | "leave-server" | "channel" | "friend" | "block";
    id: string;
    serverId?: string;
    name: string;
  } | null>(null);
  const [sidebarQuery, setSidebarQuery] = useState("");
  const [membersOpenById, setMembersOpenById] = useState<Record<string, boolean>>({});

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
    servers,
    channelsByServer,
    selectedServerId,
    selectedChannelId,
    setSelectedServer,
    setSelectedChannel,
    fetchServers,
    fetchChannels,
    voice,
  });
  const {
    selectedDm,
    setSelectedDm,
    expandedServerIds,
    collapseServer,
    selectedServer,
    activeTextChannel,
    selectedVoiceChannelNotJoined,
    selectedServerChannelsLoaded,
    serverHasTextChannel,
    onTextChannelActive,
    handleSelectChannel,
    handleSelectDm,
    finalizeJoin,
    openServer,
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
      fetchServers();
      fetchFriends();
      void useBlockStore.getState().fetch();
      void useEntitlementsStore.getState().fetch();
    }
  }, [hasSession, fetchServers, fetchFriends]);

  useVoicePresenceSync({ servers, hasSession });
  useStreamQualitySync();
  const { hasMoreDirects, loadMoreDirects } = useSidebarPrefetch({
    query: sidebarQuery,
    hasSession,
    servers,
    channelsByServer,
    fetchChannels,
  });

  const previewParticipants: ChannelParticipant[] = React.useMemo(() => {
    if (!selectedVoiceChannelNotJoined) return [];
    const members =
      voicePresenceByChannel[selectedVoiceChannelNotJoined.id] || [];
    return members.map((m) => ({ id: m.id, name: m.name }));
  }, [selectedVoiceChannelNotJoined, voicePresenceByChannel]);

  const showMembersForChannel =
    onTextChannelActive && !!activeTextChannel && !!membersOpenById[activeTextChannel.id];

  const callStageShown = useCallStore((s) => !!s.activeCallId && s.focused);
  useActiveChatView({
    textChannelId:
      onTextChannelActive && activeTextChannel ? activeTextChannel.id : null,
    dmId: selectedDm?.id ?? null,
    blocked: settingsOpen || !!serverSettings || callStageShown,
  });

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
    openServer(server.id);
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
      collapseServer(deleteConfirm.id);
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
      collapseServer(deleteConfirm.id);
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
    setSelectedDm,
    setSelectedChannel,
    setJoinedChannel,
  ]);

  const handleJoinByLink = async (token: string) => {
    const { server } = await serverAPI.joinByInvite(token);
    useServerStore.getState().addServer(server);
    openServer(server.id);
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

  const homeOverview = useHomeStore((s) => s.overview);
  const homeFailed = useHomeStore((s) => s.failed);
  const homeActive = !selectedServer && !selectedDm;
  const { fetchHome } = useHomeRefresh({
    hasSession,
    homeActive,
    watched: [voicePresenceByChannel, onlineUsers, friends],
  });

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
      selection.expandServer(serverId);
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

  const connectToPreviewChannel = (server: Server, channel: Channel) => {
    if (channel.visibility === "private") {
      setPasswordPrompt({ server, channel });
      return;
    }
    finalizeJoin(server, channel);
  };

  return (
    <div className="win">
      <div className="body-area">
        <Sidebar
          servers={servers}
          channelsByServer={channelsByServer}
          expandedServerIds={expandedServerIds}
          selectedChannelId={selectedChannelId}
          joinedChannelId={joinedChannel?.channel.id ?? null}
          voiceMembers={voiceMembers}
          voiceMembersByChannel={voiceMembersByChannel}
          selectedDmId={selectedDm?.id ?? null}
          directs={acceptedFriends}
          hasMoreDirects={hasMoreDirects}
          onLoadMoreDirects={loadMoreDirects}
          onToggleServer={selection.handleToggleServer}
          onSelectServer={selection.handleSelectServer}
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
          onGoHome={selection.handleGoHome}
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
                onGoToServer={selection.handleGoToServer}
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
              <VoiceChannelPreview
                channelName={selectedVoiceChannelNotJoined.name}
                participants={previewParticipants}
                onConnect={() =>
                  connectToPreviewChannel(
                    selectedServer,
                    selectedVoiceChannelNotJoined,
                  )
                }
                onInvite={() => setInviteModalServer(selectedServer)}
              />
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

export default ServersPage;
