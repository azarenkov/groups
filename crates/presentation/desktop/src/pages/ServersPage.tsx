import React, { useEffect, useState } from "react";
import { useAuthStore } from "../store/authStore";
import { useServerStore } from "../store/serverStore";
import { serverAPI } from "../api/server";
import ServerCard from "../components/ServerCard";
import CreateServerModal from "../components/CreateServerModal";
import "../styles/ServersPage.css";

const ServersPage: React.FC = () => {
  const { tokens, logout } = useAuthStore();
  const {
    servers,
    selectedServerId,
    isLoading,
    error,
    pagination,
    setSelectedServer,
    fetchServers,
    createServer,
    clearError,
  } = useServerStore();

  const [isModalOpen, setIsModalOpen] = useState(false);

  useEffect(() => {
    if (tokens?.accessToken) {
      serverAPI.setAuthToken(tokens.accessToken);
      fetchServers();
    }
  }, [tokens, fetchServers]);

  const handleCreateServer = async (name: string, description?: string) => {
    await createServer(name, description);
  };

  const handleLogout = () => {
    serverAPI.removeAuthToken();
    logout();
  };

  const handleLoadMore = () => {
    if (pagination && pagination.page < pagination.totalPages) {
      fetchServers(pagination.page + 1, pagination.perPage);
    }
  };

  return (
    <div className="servers-page">
      <div className="servers-sidebar">
        <div className="sidebar-header">
          <h2>Your Servers</h2>
          <button
            className="btn-create-server"
            onClick={() => setIsModalOpen(true)}
            title="Create Server"
          >
            +
          </button>
        </div>

        <div className="servers-list">
          {isLoading && servers.length === 0 ? (
            <div className="loading-state">
              <div className="spinner"></div>
              <p>Loading servers...</p>
            </div>
          ) : error ? (
            <div className="error-state">
              <p className="error-text">{error}</p>
              <button
                className="btn-retry"
                onClick={() => {
                  clearError();
                  fetchServers();
                }}
              >
                Retry
              </button>
            </div>
          ) : servers.length === 0 ? (
            <div className="empty-state">
              <p>No servers yet</p>
              <p className="empty-state-subtitle">
                Create your first server to get started
              </p>
            </div>
          ) : (
            <>
              {servers.map((server) => (
                <ServerCard
                  key={server.id}
                  server={server}
                  isSelected={selectedServerId === server.id}
                  onSelect={setSelectedServer}
                />
              ))}
              {pagination && pagination.page < pagination.totalPages && (
                <button
                  className="btn-load-more"
                  onClick={handleLoadMore}
                  disabled={isLoading}
                >
                  {isLoading ? "Loading..." : "Load More"}
                </button>
              )}
            </>
          )}
        </div>

        <div className="sidebar-footer">
          <button className="btn-logout" onClick={handleLogout}>
            Logout
          </button>
        </div>
      </div>

      <div className="servers-content">
        {selectedServerId ? (
          <div className="server-details">
            <h1>Server Details</h1>
            <p>Server ID: {selectedServerId}</p>
            <p className="coming-soon">Rooms and channels coming soon...</p>
          </div>
        ) : (
          <div className="no-server-selected">
            <h2>Welcome to Rift</h2>
            <p>Select a server from the list or create a new one</p>
          </div>
        )}
      </div>

      <CreateServerModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onCreate={handleCreateServer}
      />
    </div>
  );
};

export default ServersPage;
