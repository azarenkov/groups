import React, { useEffect, useState } from "react";
import { useAuthStore } from "../store/authStore";
import { useServerStore } from "../store/serverStore";
import { useRoomStore } from "../store/roomStore";
import { serverAPI } from "../api/server";
import { roomAPI } from "../api/room";
import CreateServerModal from "../components/CreateServerModal";
import CreateRoomModal from "../components/CreateRoomModal";
import VoiceRoom from "../components/VoiceRoom";
import { RoomType } from "../types/room";
import "../styles/ServersPage.css";

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
    isLoading: roomsLoading,
    error: roomsError,
    setSelectedRoom,
    fetchRooms,
    createRoom,
    deleteRoom,
    clearError: clearRoomsError,
    clearRooms,
  } = useRoomStore();

  const [isServerModalOpen, setIsServerModalOpen] = useState(false);
  const [isRoomModalOpen, setIsRoomModalOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    type: "server" | "room";
    id: string;
    name: string;
  } | null>(null);
  const [joinedRoomId, setJoinedRoomId] = useState<string | null>(null);

  useEffect(() => {
    if (tokens?.accessToken) {
      serverAPI.setAuthToken(tokens.accessToken);
      roomAPI.setAuthToken(tokens.accessToken);
      fetchServers();
    }
  }, [tokens, fetchServers]);

  useEffect(() => {
    if (selectedServerId && tokens?.accessToken) {
      fetchRooms(selectedServerId);
    }
  }, [selectedServerId, tokens, fetchRooms]);

  const handleCreateServer = async (name: string, description?: string) => {
    await createServer(name, description);
  };

  const handleCreateRoom = async (
    name: string,
    roomType: RoomType,
    password?: string,
  ) => {
    if (selectedServerId) {
      await createRoom(selectedServerId, name, roomType, password);
    }
  };

  const handleSelectServer = (serverId: string) => {
    setSelectedServer(serverId);
    setSelectedRoom(null);
  };

  const handleDeleteServer = async () => {
    if (deleteConfirm?.type === "server" && deleteConfirm.id) {
      try {
        await deleteServer(deleteConfirm.id);
        if (selectedServerId === deleteConfirm.id) {
          setSelectedServer(null);
          clearRooms(deleteConfirm.id);
        }
        setDeleteConfirm(null);
      } catch (error) {
        console.error("Failed to delete server:", error);
      }
    }
  };

  const handleDeleteRoom = async () => {
    if (
      deleteConfirm?.type === "room" &&
      deleteConfirm.id &&
      selectedServerId
    ) {
      try {
        await deleteRoom(selectedServerId, deleteConfirm.id);
        if (selectedRoomId === deleteConfirm.id) {
          setSelectedRoom(null);
        }
        setDeleteConfirm(null);
      } catch (error) {
        console.error("Failed to delete room:", error);
      }
    }
  };

  const handleLogout = () => {
    serverAPI.removeAuthToken();
    roomAPI.removeAuthToken();
    logout();
  };

  const selectedServer = servers.find((s) => s.id === selectedServerId);
  const currentRooms = selectedServerId
    ? roomsByServer[selectedServerId] || []
    : [];
  const selectedRoom = currentRooms.find((r) => r.id === selectedRoomId);

  const error = serversError || roomsError;
  const clearError = () => {
    clearServersError();
    clearRoomsError();
  };

  return (
    <div className="discord-layout">
      {/* Left Sidebar - Server Icons */}
      <div className="server-list-sidebar">
        <div className="server-list-content">
          {/* Home Button */}
          <div
            className={`server-icon-wrapper ${!selectedServerId ? "active" : ""}`}
            onClick={() => {
              setSelectedServer(null);
              setSelectedRoom(null);
            }}
            title="Home"
          >
            <div className="server-icon home-icon">
              <svg width="28" height="28" viewBox="0 0 28 28">
                <path
                  fill="currentColor"
                  d="M14 2L2 10v12h8v-8h4v8h8V10L14 2z"
                />
              </svg>
            </div>
          </div>

          <div className="server-separator"></div>

          {/* Server Icons */}
          {serversLoading && servers.length === 0 ? (
            <div className="server-icon-wrapper">
              <div className="server-icon loading-icon">
                <div className="spinner-small"></div>
              </div>
            </div>
          ) : (
            servers.map((server) => (
              <div
                key={server.id}
                className={`server-icon-wrapper ${
                  selectedServerId === server.id ? "active" : ""
                }`}
                onClick={() => handleSelectServer(server.id)}
                onContextMenu={(e) => {
                  e.preventDefault();
                  setDeleteConfirm({
                    type: "server",
                    id: server.id,
                    name: server.name,
                  });
                }}
                title={server.name}
              >
                <div className="server-icon">
                  {server.name.charAt(0).toUpperCase()}
                </div>
                {selectedServerId === server.id && (
                  <div className="active-indicator"></div>
                )}
              </div>
            ))
          )}

          {/* Add Server Button */}
          <div
            className="server-icon-wrapper add-server"
            onClick={() => setIsServerModalOpen(true)}
            title="Add a Server"
          >
            <div className="server-icon add-icon">+</div>
          </div>
        </div>
      </div>

      {/* Middle Panel - Channels/Rooms */}
      <div className="channels-sidebar">
        <div className="channels-header">
          <h2 className="server-name">
            {selectedServer ? selectedServer.name : "Home"}
          </h2>
          {selectedServer && (
            <button
              className="server-dropdown-btn"
              title="Delete Server"
              onClick={() =>
                setDeleteConfirm({
                  type: "server",
                  id: selectedServer.id,
                  name: selectedServer.name,
                })
              }
            >
              <svg width="18" height="18" viewBox="0 0 24 24">
                <path
                  fill="currentColor"
                  d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"
                />
              </svg>
            </button>
          )}
        </div>

        <div className="channels-content">
          {!selectedServer ? (
            <div className="home-content">
              <div className="user-section">
                <div className="user-avatar">
                  {user?.name.charAt(0).toUpperCase()}
                </div>
                <div className="user-info">
                  <div className="user-name">{user?.name}</div>
                  <div className="user-status">Online</div>
                </div>
              </div>

              <div className="home-section">
                <h3 className="section-title">YOUR SERVERS</h3>
                {servers.length === 0 ? (
                  <div className="empty-servers">
                    <p>No servers yet</p>
                    <button
                      className="btn-create-first"
                      onClick={() => setIsServerModalOpen(true)}
                    >
                      Create Server
                    </button>
                  </div>
                ) : (
                  <div className="server-list-items">
                    {servers.map((server) => (
                      <div
                        key={server.id}
                        className="server-list-item"
                        onClick={() => handleSelectServer(server.id)}
                      >
                        <div className="server-item-icon">
                          {server.name.charAt(0).toUpperCase()}
                        </div>
                        <div className="server-item-info">
                          <div className="server-item-name">{server.name}</div>
                          {server.description && (
                            <div className="server-item-description">
                              {server.description}
                            </div>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ) : (
            <>
              <div className="channel-category">
                <div className="category-header">
                  <svg width="12" height="12" viewBox="0 0 24 24">
                    <path
                      fill="currentColor"
                      d="M9.3 5.3a1 1 0 0 0 0 1.4l5.29 5.3-5.3 5.3a1 1 0 1 0 1.42 1.4l6-6a1 1 0 0 0 0-1.4l-6-6a1 1 0 0 0-1.42 0z"
                    />
                  </svg>
                  <span>TEXT CHANNELS</span>
                </div>
              </div>

              <div className="channels-list">
                <div className="channel-item">
                  <span className="channel-hash">#</span>
                  <span>general</span>
                </div>
              </div>

              <div className="channel-category">
                <div
                  className="category-header"
                  style={{ justifyContent: "space-between" }}
                >
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "4px",
                    }}
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24">
                      <path
                        fill="currentColor"
                        d="M9.3 5.3a1 1 0 0 0 0 1.4l5.29 5.3-5.3 5.3a1 1 0 1 0 1.42 1.4l6-6a1 1 0 0 0 0-1.4l-6-6a1 1 0 0 0-1.42 0z"
                      />
                    </svg>
                    <span>ROOMS</span>
                  </div>
                  <button
                    className="add-channel-btn"
                    onClick={() => setIsRoomModalOpen(true)}
                    title="Create Room"
                  >
                    +
                  </button>
                </div>
              </div>

              <div className="channels-list">
                {roomsLoading && currentRooms.length === 0 ? (
                  <div className="channel-item">
                    <div className="spinner-small"></div>
                    <span>Loading...</span>
                  </div>
                ) : currentRooms.length === 0 ? (
                  <div className="empty-channels">No rooms yet</div>
                ) : (
                  currentRooms.map((room) => (
                    <div
                      key={room.id}
                      className={`channel-item voice ${joinedRoomId === room.id ? "active" : ""}`}
                      onClick={() => setJoinedRoomId(room.id)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setDeleteConfirm({
                          type: "room",
                          id: room.id,
                          name: room.name,
                        });
                      }}
                    >
                      <svg width="20" height="20" viewBox="0 0 24 24">
                        <path
                          fill="currentColor"
                          d="M12 3a9 9 0 0 0-9 9v7a2 2 0 0 0 2 2h2v-5H5v-4a7 7 0 1 1 14 0v4h-2v5h2a2 2 0 0 0 2-2v-7a9 9 0 0 0-9-9z"
                        />
                      </svg>
                      <span>{room.name}</span>
                      {room.roomType === "private" && (
                        <svg
                          width="16"
                          height="16"
                          viewBox="0 0 24 24"
                          style={{ marginLeft: "auto" }}
                        >
                          <path
                            fill="currentColor"
                            d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zM9 6c0-1.66 1.34-3 3-3s3 1.34 3 3v2H9V6z"
                          />
                        </svg>
                      )}
                      <span className="participant-count">
                        {room.maxParticipants}
                      </span>
                    </div>
                  ))
                )}
              </div>
            </>
          )}
        </div>

        {/* User Panel */}
        <div className="user-panel">
          <div className="user-panel-content">
            <div className="user-panel-avatar">
              {user?.name.charAt(0).toUpperCase()}
            </div>
            <div className="user-panel-info">
              <div className="user-panel-name">{user?.name}</div>
              <div className="user-panel-status">Online</div>
            </div>
          </div>
          <div className="user-panel-actions">
            <button className="user-action-btn" title="Settings">
              <svg width="20" height="20" viewBox="0 0 24 24">
                <path
                  fill="currentColor"
                  d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.488.488 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"
                />
              </svg>
            </button>
            <button
              className="user-action-btn logout-btn"
              onClick={handleLogout}
              title="Logout"
            >
              <svg width="20" height="20" viewBox="0 0 24 24">
                <path
                  fill="currentColor"
                  d="M16 17v-3H9v-4h7V7l5 5-5 5M14 2a2 2 0 0 1 2 2v2h-2V4H5v16h9v-2h2v2a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9z"
                />
              </svg>
            </button>
          </div>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="main-content">
        {!selectedServer ? (
          <div className="welcome-screen">
            <div className="welcome-content">
              <h1>Welcome to Rift</h1>
              <p>
                Select a server from the left or create a new one to get started
              </p>
            </div>
          </div>
        ) : selectedRoom ? (
          <div className="chat-area">
            <div className="chat-header">
              <svg width="24" height="24" viewBox="0 0 24 24">
                <path
                  fill="currentColor"
                  d="M12 3a9 9 0 0 0-9 9v7a2 2 0 0 0 2 2h2v-5H5v-4a7 7 0 1 1 14 0v4h-2v5h2a2 2 0 0 0 2-2v-7a9 9 0 0 0-9-9z"
                />
              </svg>
              <span className="channel-name">{selectedRoom.name}</span>
              <div className="chat-header-divider"></div>
              <span className="channel-description">
                {selectedRoom.roomType === "private" ? "🔒 Private" : "Public"}{" "}
                • Max {selectedRoom.maxParticipants} participants
              </span>
            </div>
            <div className="chat-messages">
              <div className="messages-start">
                <div className="messages-start-icon">
                  <svg width="48" height="48" viewBox="0 0 24 24">
                    <path
                      fill="currentColor"
                      d="M12 3a9 9 0 0 0-9 9v7a2 2 0 0 0 2 2h2v-5H5v-4a7 7 0 1 1 14 0v4h-2v5h2a2 2 0 0 0 2-2v-7a9 9 0 0 0-9-9z"
                    />
                  </svg>
                </div>
                <h2>Welcome to {selectedRoom.name}</h2>
                <p>
                  This is a voice room. Voice chat functionality coming soon!
                </p>
              </div>
            </div>
          </div>
        ) : (
          <div className="chat-area">
            <div className="chat-header">
              <span className="channel-hash">#</span>
              <span className="channel-name">general</span>
              <div className="chat-header-divider"></div>
              <span className="channel-description">
                Main discussion channel
              </span>
            </div>
            <div className="chat-messages">
              <div className="messages-start">
                <div className="messages-start-icon">#</div>
                <h2>Welcome to #{selectedServer.name}</h2>
                <p>This is the beginning of the #general channel.</p>
              </div>
            </div>
            <div className="chat-input-container">
              <div className="chat-input-wrapper">
                <input
                  type="text"
                  className="chat-input"
                  placeholder={`Message #general`}
                  disabled
                />
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Modals */}
      <CreateServerModal
        isOpen={isServerModalOpen}
        onClose={() => setIsServerModalOpen(false)}
        onCreate={handleCreateServer}
      />

      <CreateRoomModal
        isOpen={isRoomModalOpen}
        onClose={() => setIsRoomModalOpen(false)}
        onCreate={handleCreateRoom}
      />

      {/* Delete Confirmation Modal */}
      {deleteConfirm && (
        <div className="modal-overlay" onClick={() => setDeleteConfirm(null)}>
          <div
            className="modal-content delete-modal"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="modal-header">
              <h2>
                Delete {deleteConfirm.type === "server" ? "Server" : "Room"}
              </h2>
              <button
                className="modal-close-btn"
                onClick={() => setDeleteConfirm(null)}
              >
                ×
              </button>
            </div>
            <div className="modal-form">
              <p className="delete-warning">
                Are you sure you want to delete{" "}
                <strong>{deleteConfirm.name}</strong>? This action cannot be
                undone.
              </p>
              <div className="modal-actions">
                <button
                  className="btn-secondary"
                  onClick={() => setDeleteConfirm(null)}
                >
                  Cancel
                </button>
                <button
                  className="btn-danger"
                  onClick={
                    deleteConfirm.type === "server"
                      ? handleDeleteServer
                      : handleDeleteRoom
                  }
                >
                  Delete
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Error Toast */}
      {error && (
        <div className="error-toast">
          <span>{error}</span>
          <button onClick={clearError}>×</button>
        </div>
      )}

      {/* Voice Room Component */}
      {joinedRoomId && selectedServerId && (
        <VoiceRoom
          roomId={joinedRoomId}
          serverId={selectedServerId}
          roomName={
            currentRooms.find((r) => r.id === joinedRoomId)?.name || "Room"
          }
          onClose={() => setJoinedRoomId(null)}
        />
      )}
    </div>
  );
};

export default ServersPage;
