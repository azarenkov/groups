import React, { useEffect, useState } from "react";
import { useAuthStore } from "../store/authStore";
import { useServerStore } from "../store/serverStore";
import { useRoomStore } from "../store/roomStore";
import { useThemeStore } from "../store/themeStore";
import { serverAPI } from "../api/server";
import { roomAPI } from "../api/room";
import CreateServerModal from "../components/CreateServerModal";
import CreateRoomModal from "../components/CreateRoomModal";
import NewDmModal from "../components/NewDmModal";
import ChannelSettingsModal from "../components/ChannelSettingsModal";
import ConfirmModal from "../components/rift/ConfirmModal";
import Sidebar from "../components/rift/Sidebar";
import ChatView from "../components/rift/ChatView";
import VoiceView from "../components/rift/VoiceView";
import MembersPanel from "../components/rift/MembersPanel";
import SettingsView, { TweaksPanel } from "../components/settings/SettingsView";
import Icon from "../components/rift/Icon";
import { Room, RoomType } from "../types/room";
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
  const { tokens, user, logout } = useAuthStore();
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
    roomsByServer,
    selectedRoomId,
    error: roomsError,
    setSelectedRoom,
    fetchRooms,
    createRoom,
    deleteRoom,
    clearError: clearRoomsError,
    clearRooms,
  } = useRoomStore();

  const { theme, setTheme } = useThemeStore();
  const [accent, setAccent] = useState("#0071e3");

  // Apply accent CSS var
  useEffect(() => {
    document.documentElement.style.setProperty("--accent", accent);
    document.documentElement.style.setProperty(
      "--accent-hover",
      `color-mix(in srgb, ${accent} 88%, #000)`,
    );
  }, [accent]);

  // Cmd+, opens settings
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

  // Modals + side panels
  const [isServerModalOpen, setIsServerModalOpen] = useState(false);
  const [isRoomModalOpen, setIsRoomModalOpen] = useState(false);
  const [roomModalServerId, setRoomModalServerId] = useState<string | null>(null);
  const [isDmModalOpen, setIsDmModalOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [tweaksOpen, setTweaksOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    type: "server" | "room";
    id: string;
    serverId?: string;
    name: string;
  } | null>(null);

  // Conversation state — exactly one of (joinedRoom | selectedDm | text-channel) is active
  const [joinedRoom, setJoinedRoom] = useState<{ room: Room; serverId: string } | null>(null);
  const [selectedDm, setSelectedDm] = useState<MockDirect | null>(null);
  const [expandedServerId, setExpandedServerId] = useState<string | null>(null);
  const [directs, setDirects] = useState<MockDirect[]>(SEED_DIRECTS);

  // Channel overrides (text channel name/topic edits) keyed by `${serverId}:general`
  const [channelOverrides, setChannelOverrides] =
    useState<Record<string, ChannelOverride>>({});
  const [editingChannel, setEditingChannel] = useState<{
    id: string;
    name: string;
    topic?: string;
  } | null>(null);

  // Members panel toggle (per text-channel id)
  const [membersOpenById, setMembersOpenById] = useState<Record<string, boolean>>({});

  useEffect(() => {
    if (tokens?.accessToken) {
      serverAPI.setAuthToken(tokens.accessToken);
      roomAPI.setAuthToken(tokens.accessToken);
      fetchServers();
    }
  }, [tokens, fetchServers]);

  useEffect(() => {
    if (expandedServerId && tokens?.accessToken && !roomsByServer[expandedServerId]) {
      fetchRooms(expandedServerId);
    }
  }, [expandedServerId, tokens, fetchRooms, roomsByServer]);

  useEffect(() => {
    if (!expandedServerId && servers.length > 0) {
      setExpandedServerId(servers[0].id);
      setSelectedServer(servers[0].id);
    }
  }, [servers, expandedServerId, setSelectedServer]);

  const selectedServer = servers.find((s) => s.id === selectedServerId) || null;

  // Effective text-channel meta for the active server's "general"
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

  const onTextChannelActive =
    !!selectedServer && !joinedRoom && !selectedDm;
  const showMembersForChannel =
    onTextChannelActive && !!generalChannel && !!membersOpenById[generalChannel.id];

  const handleToggleServer = (serverId: string) => {
    setExpandedServerId((prev) => (prev === serverId ? null : serverId));
  };

  const handleSelectServer = (serverId: string) => {
    setSelectedServer(serverId);
    setSelectedRoom(null);
    setJoinedRoom(null);
    setSelectedDm(null);
  };

  const handleSelectRoom = (server: Server, room: Room) => {
    setSelectedServer(server.id);
    setSelectedRoom(room.id);
    setJoinedRoom({ room, serverId: server.id });
    setSelectedDm(null);
  };

  const handleSelectDm = (dm: MockDirect) => {
    setSelectedDm(dm);
    setJoinedRoom(null);
    setSelectedRoom(null);
  };

  const handleCreateServer = async (name: string, description?: string) => {
    await createServer(name, description);
  };

  const handleCreateRoom = async (
    name: string,
    roomType: RoomType,
    password?: string,
  ) => {
    if (roomModalServerId) {
      await createRoom(roomModalServerId, name, roomType, password);
    }
  };

  const handleDeleteServer = async () => {
    if (deleteConfirm?.type !== "server") return;
    try {
      await deleteServer(deleteConfirm.id);
      if (selectedServerId === deleteConfirm.id) {
        setSelectedServer(null);
        clearRooms(deleteConfirm.id);
      }
      if (expandedServerId === deleteConfirm.id) setExpandedServerId(null);
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to delete server:", err);
    }
  };

  const handleDeleteRoom = async () => {
    if (deleteConfirm?.type !== "room" || !deleteConfirm.serverId) return;
    try {
      await deleteRoom(deleteConfirm.serverId, deleteConfirm.id);
      if (selectedRoomId === deleteConfirm.id) {
        setSelectedRoom(null);
        setJoinedRoom(null);
      }
      setDeleteConfirm(null);
    } catch (err) {
      console.error("Failed to delete room:", err);
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

  const handleLogout = () => {
    serverAPI.removeAuthToken();
    roomAPI.removeAuthToken();
    logout();
  };

  const error = serversError || roomsError;
  const clearError = () => {
    clearServersError();
    clearRoomsError();
  };

  const userName = user?.name || "You";
  const userStatus = "Online";

  const winTitle = selectedDm
    ? selectedDm.name
    : joinedRoom
      ? `${servers.find((s) => s.id === joinedRoom.serverId)?.name ?? ""} — ${joinedRoom.room.name}`
      : selectedServer
        ? `${selectedServer.name} — ${generalChannel?.name ?? "general"}`
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
          roomsByServer={roomsByServer}
          expandedServerId={expandedServerId}
          selectedServerId={selectedServerId}
          selectedRoomId={joinedRoom?.room.id ?? null}
          selectedDmId={selectedDm?.id ?? null}
          directs={directs}
          onToggleServer={handleToggleServer}
          onSelectServer={handleSelectServer}
          onSelectRoom={handleSelectRoom}
          onSelectDm={handleSelectDm}
          onAddServer={() => setIsServerModalOpen(true)}
          onAddRoom={(server) => {
            setRoomModalServerId(server.id);
            setIsRoomModalOpen(true);
          }}
          onAddDm={() => setIsDmModalOpen(true)}
          onDeleteServer={(server) =>
            setDeleteConfirm({ type: "server", id: server.id, name: server.name })
          }
          onDeleteRoom={(server, room) =>
            setDeleteConfirm({
              type: "room",
              id: room.id,
              serverId: server.id,
              name: room.name,
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
                onLogout={handleLogout}
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
            ) : (
              <ChatView
                channel={{
                  name: joinedRoom ? joinedRoom.room.name : generalChannel?.name ?? "general",
                  topic: joinedRoom
                    ? `${selectedServer?.name ?? ""} · voice room`
                    : generalChannel?.topic,
                  type: joinedRoom ? "voice" : "text",
                }}
                membersOpen={
                  onTextChannelActive && !!generalChannel
                    ? !!membersOpenById[generalChannel.id]
                    : false
                }
                onToggleMembers={
                  onTextChannelActive && generalChannel
                    ? () =>
                        setMembersOpenById((prev) => ({
                          ...prev,
                          [generalChannel.id]: !prev[generalChannel.id],
                        }))
                    : undefined
                }
                onOpenChannelSettings={
                  onTextChannelActive && generalChannel
                    ? () => setEditingChannel(generalChannel)
                    : undefined
                }
              />
            )}

            {joinedRoom && (
              <VoiceView
                key={joinedRoom.room.id}
                roomId={joinedRoom.room.id}
                serverId={joinedRoom.serverId}
                roomName={joinedRoom.room.name}
                currentUserName={userName}
                onLeave={() => {
                  setJoinedRoom(null);
                  setSelectedRoom(null);
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

      <CreateRoomModal
        isOpen={isRoomModalOpen}
        onClose={() => {
          setIsRoomModalOpen(false);
          setRoomModalServerId(null);
        }}
        onCreate={handleCreateRoom}
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
            // No backend for "delete general channel" — close modal as no-op
            setEditingChannel(null);
          }}
        />
      )}

      {deleteConfirm && (
        <ConfirmModal
          title={`Delete ${deleteConfirm.type === "server" ? "server" : "room"}?`}
          subtitle="This can't be undone."
          targetName={deleteConfirm.name}
          confirmLabel="Delete"
          danger
          onCancel={() => setDeleteConfirm(null)}
          onConfirm={
            deleteConfirm.type === "server" ? handleDeleteServer : handleDeleteRoom
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
  onLogout: () => void;
}

const Welcome: React.FC<WelcomeProps> = ({ loading, onCreate, onLogout }) => (
  <div className="empty-state" style={{ padding: 60 }}>
    <div className="icon-wrap" style={{ width: 64, height: 64 }}>
      <Icon name="hash" size={26} />
    </div>
    <h3>Welcome to Rift</h3>
    <p>Pick a server on the left or create one to start talking.</p>
    <div style={{ display: "flex", gap: 8, marginTop: 16 }}>
      {loading ? (
        <span className="loading-spinner" />
      ) : (
        <button
          type="button"
          className="btn primary"
          onClick={onCreate}
          style={{ height: 36, padding: "0 14px" }}
        >
          <Icon name="plus" size={14} />
          Create a server
        </button>
      )}
      <button
        type="button"
        onClick={onLogout}
        style={{
          height: 36,
          padding: "0 14px",
          background: "transparent",
          color: "var(--fg-3)",
          borderRadius: 7,
          fontSize: 13,
          display: "inline-flex",
          alignItems: "center",
          gap: 6,
        }}
      >
        <Icon name="logout" size={14} />
        Sign out
      </button>
    </div>
  </div>
);

export default ServersPage;
