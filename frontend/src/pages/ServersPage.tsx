import React, { useEffect, useRef, useState } from "react";
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
import ChannelSettingsModal from "../components/ChannelSettingsModal";
import ConfirmModal from "../components/rift/ConfirmModal";
import Sidebar from "../components/rift/Sidebar";
import ChatView from "../components/rift/ChatView";
import VoiceView from "../components/rift/VoiceView";
import MembersPanel from "../components/rift/MembersPanel";
import SettingsView, { TweaksPanel } from "../components/settings/SettingsView";
import Icon from "../components/rift/Icon";
import { Channel, ChannelKind, ChannelVisibility } from "../types/channel";
import { Server } from "../types/server";
import {
  SEED_DIRECTS,
  SEED_MEMBERS,
  type MockDirect,
  type MockMember,
} from "../data/mockData";
import "../styles/rift.css";

interface ChannelOverride {
  name?: string;
  topic?: string;
}

const ServersPage: React.FC = () => {
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
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [tweaksOpen, setTweaksOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    type: "server" | "channel";
    id: string;
    serverId?: string;
    name: string;
  } | null>(null);

  const [joinedChannel, setJoinedChannel] = useState<{
    channel: Channel;
    serverId: string;
    password?: string;
  } | null>(null);
  const [passwordPrompt, setPasswordPrompt] = useState<{
    server: Server;
    channel: Channel;
  } | null>(null);
  const [selectedDm, setSelectedDm] = useState<MockDirect | null>(null);
  const [expandedServerId, setExpandedServerId] = useState<string | null>(null);
  const [directs, setDirects] = useState<MockDirect[]>(SEED_DIRECTS);

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
      fetchServers();
    }
  }, [tokens, fetchServers]);

  useEffect(() => {
    if (expandedServerId && tokens?.accessToken && !channelsByServer[expandedServerId]) {
      fetchChannels(expandedServerId);
    }
  }, [expandedServerId, tokens, fetchChannels, channelsByServer]);

  const didInitialExpandRef = useRef(false);
  useEffect(() => {
    if (didInitialExpandRef.current) return;
    if (servers.length === 0) return;
    didInitialExpandRef.current = true;
    setExpandedServerId(servers[0].id);
    setSelectedServer(servers[0].id);
  }, [servers, setSelectedServer]);

  useEffect(() => {
    if (!selectedServerId || selectedChannelId || selectedDm) return;
    const list = channelsByServer[selectedServerId];
    if (!list) return;
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

  const generalChannelKey = selectedServer ? `${selectedServer.id}:general` : "";
  const generalOverride = channelOverrides[generalChannelKey] || {};
  const generalChannel = selectedServer
    ? {
        id: generalChannelKey,
        name: generalOverride.name ?? "general",
        topic:
          generalOverride.topic ?? (selectedServer.description || selectedServer.name),
      }
    : null;

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
    : generalChannel;

  const viewingJoinedVoice =
    !!joinedChannel && selectedChannelId === joinedChannel.channel.id;
  const onTextChannelActive =
    !!selectedServer && !selectedDm && !viewingJoinedVoice;
  const showMembersForChannel =
    onTextChannelActive && !!activeTextChannel && !!membersOpenById[activeTextChannel.id];

  const handleToggleServer = (serverId: string) => {
    setExpandedServerId((prev) => (prev === serverId ? null : serverId));
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
  };

  const handleSelectChannel = (server: Server, channel: Channel) => {
    if (channel.kind === "text") {
      setSelectedServer(server.id);
      setSelectedChannel(channel.id);
      setSelectedDm(null);
      return;
    }
    if (joinedChannel && joinedChannel.channel.id === channel.id) {
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

  const handleCreateServer = async (name: string, description?: string) => {
    const server = await createServer(name, description);
    try {
      await createChannel(server.id, "general", "text", "public");
    } catch (err) {
      console.error("[ServersPage] failed to create default channel:", err);
    }
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
      if (expandedServerId === deleteConfirm.id) setExpandedServerId(null);
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to delete server:", err);
    }
  };

  const handleDeleteChannel = async () => {
    if (deleteConfirm?.type !== "channel" || !deleteConfirm.serverId) return;
    try {
      await deleteChannel(deleteConfirm.serverId, deleteConfirm.id);
      if (selectedChannelId === deleteConfirm.id) {
        setSelectedChannel(null);
        setJoinedChannel(null);
      }
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to delete channel:", err);
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

  const handleLogout = async () => {
    serverAPI.removeAuthToken();
    channelAPI.removeAuthToken();
    await signOut();
  };

  const error = serversError || channelsError;
  const clearError = () => {
    clearServersError();
    clearChannelsError();
  };

  const userName = user?.name || "You";
  const userStatus = "Online";

  const winTitle = selectedDm
    ? selectedDm.name
    : joinedChannel
      ? `${servers.find((s) => s.id === joinedChannel.serverId)?.name ?? ""} — ${joinedChannel.channel.name}`
      : selectedServer
        ? `${selectedServer.name} — ${activeTextChannel?.name ?? "general"}`
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
        <button
          type="button"
          className="icon-btn"
          title="Tweaks"
          style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
          onClick={() => setTweaksOpen((v) => !v)}
        >
          <Icon name="palette" size={14} />
        </button>
      </div>

      <div className="body-area">
        <Sidebar
          servers={servers}
          channelsByServer={channelsByServer}
          expandedServerId={expandedServerId}
          selectedChannelId={selectedChannelId}
          joinedChannelId={joinedChannel?.channel.id ?? null}
          selectedDmId={selectedDm?.id ?? null}
          directs={directs}
          onToggleServer={handleToggleServer}
          onSelectServer={handleSelectServer}
          onSelectChannel={handleSelectChannel}
          onSelectDm={handleSelectDm}
          onAddServer={() => setIsServerModalOpen(true)}
          onAddChannel={(server) => {
            setChannelModalServerId(server.id);
            setIsChannelModalOpen(true);
          }}
          onAddDm={() => setIsDmModalOpen(true)}
          onDeleteServer={(server) =>
            setDeleteConfirm({ type: "server", id: server.id, name: server.name })
          }
          onDeleteChannel={(server, channel) =>
            setDeleteConfirm({
              type: "channel",
              id: channel.id,
              serverId: server.id,
              name: channel.name,
            })
          }
          onOpenSettings={() => setSettingsOpen(true)}
          currentUserName={userName}
          currentUserStatus={userStatus}
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
                  name: selectedDm.name,
                  initials: selectedDm.initials,
                  color: selectedDm.color,
                  online: selectedDm.online,
                }}
              />
            ) : joinedChannel && selectedChannelId === joinedChannel.channel.id ? (
              <div className="voice-stage">
                <button
                  type="button"
                  className="voice-stage-settings"
                  title="Channel settings"
                  onClick={() =>
                    setEditingChannel({
                      id: joinedChannel.channel.id,
                      serverId: joinedChannel.serverId,
                      name: joinedChannel.channel.name,
                    })
                  }
                >
                  <Icon name="settings" size={14} />
                </button>
                <div className="voice-stage-orb">
                  <span className="voice-stage-orb-ring" />
                  <span className="voice-stage-orb-ring delay" />
                  <span className="voice-stage-orb-letter">
                    {joinedChannel.channel.name.charAt(0).toUpperCase()}
                  </span>
                </div>
                <h1 className="voice-stage-title">{joinedChannel.channel.name}</h1>
                <div className="voice-stage-sub">
                  <span className="voice-stage-dot" />
                  Connected
                  {selectedServer ? ` · ${selectedServer.name}` : ""}
                </div>
              </div>
            ) : (
              <ChatView
                channel={{
                  name: activeTextChannel?.name ?? "general",
                  topic: activeTextChannel?.topic,
                  type: "text",
                }}
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
                  activeTextChannel && selectedServer
                    ? () =>
                        setEditingChannel({
                          id: activeTextChannel.id,
                          serverId: selectedServer.id,
                          name: activeTextChannel.name,
                          topic: activeTextChannel.topic,
                        })
                    : undefined
                }
              />
            )}

            {joinedChannel && (
              <VoiceView
                key={joinedChannel.channel.id}
                channelId={joinedChannel.channel.id}
                serverId={joinedChannel.serverId}
                channelName={joinedChannel.channel.name}
                password={joinedChannel.password}
                currentUserName={userName}
                onLeave={() => {
                  setJoinedChannel(null);
                  setSelectedChannel(null);
                }}
              />
            )}
          </div>

          {showMembersForChannel && <MembersPanel members={SEED_MEMBERS} />}
        </main>

        {settingsOpen && (
          <SettingsView
            open={settingsOpen}
            theme={theme}
            onTheme={setTheme}
            accent={accent}
            onAccent={setAccent}
            onClose={() => setSettingsOpen(false)}
            onSignOut={() => {
              setSettingsOpen(false);
              handleLogout();
            }}
          />
        )}
      </div>

      {tweaksOpen && <TweaksPanel theme={theme} onTheme={setTheme} />}

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
          title={`Delete ${deleteConfirm.type === "server" ? "server" : "channel"}?`}
          subtitle="This can't be undone."
          targetName={deleteConfirm.name}
          confirmLabel="Delete"
          danger
          onCancel={() => setDeleteConfirm(null)}
          onConfirm={
            deleteConfirm.type === "server" ? handleDeleteServer : handleDeleteChannel
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

export default ServersPage;
