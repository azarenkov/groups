import React from "react";
import Icon from "./Icon";
import type { Server } from "../../types/server";
import type { Channel } from "../../types/channel";
import type { MockDirect } from "../../data/mockData";

interface SidebarProps {
  servers: Server[];
  channelsByServer: Record<string, Channel[]>;
  expandedServerId: string | null;
  selectedServerId: string | null;
  selectedChannelId: string | null;
  joinedChannelId: string | null;
  selectedDmId: string | null;
  directs: MockDirect[];
  onToggleServer: (serverId: string) => void;
  onSelectServer: (serverId: string) => void;
  onSelectChannel: (server: Server, channel: Channel) => void;
  onSelectDm: (dm: MockDirect) => void;
  onAddServer: () => void;
  onAddChannel: (server: Server) => void;
  onAddDm: () => void;
  onDeleteServer: (server: Server) => void;
  onDeleteChannel: (server: Server, channel: Channel) => void;
  onOpenSettings: () => void;
  currentUserName: string;
  currentUserStatus: string;
}

const accentFor = (id: string): string => {
  const PALETTE = ["#0071e3", "#30a46c", "#ff9f0a", "#bf5af2", "#ff6b35", "#5b6cd9", "#d97557", "#30d158"];
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
  return PALETTE[h % PALETTE.length];
};

const Sidebar: React.FC<SidebarProps> = ({
  servers,
  channelsByServer,
  expandedServerId,
  selectedServerId,
  selectedChannelId,
  joinedChannelId,
  selectedDmId,
  directs,
  onToggleServer,
  onSelectServer,
  onSelectChannel,
  onSelectDm,
  onAddServer,
  onAddChannel,
  onAddDm,
  onDeleteServer,
  onDeleteChannel,
  onOpenSettings,
  currentUserName,
  currentUserStatus,
}) => {
  const [collapsed, setCollapsed] = React.useState<Record<string, boolean>>({});
  const toggleSection = (id: string) =>
    setCollapsed((c) => ({ ...c, [id]: !c[id] }));

  const directsCollapsed = !!collapsed["__directs"];

  return (
    <div className="sb">
      <div className="sb-search">
        <Icon name="search" size={14} />
        <input placeholder="Search" />
        <kbd>⌘K</kbd>
      </div>

      <div className="sb-scroll">
        {/* Direct messages */}
        <div
          className={"sb-section-head" + (directsCollapsed ? " collapsed" : "")}
          onClick={() => toggleSection("__directs")}
        >
          <span>Direct Messages</span>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            <button
              type="button"
              className="sb-section-add"
              title="New direct message"
              onClick={(e) => {
                e.stopPropagation();
                onAddDm();
              }}
            >
              <Icon name="plus" size={14} />
            </button>
            <Icon name="chev" size={12} className="chev" />
          </span>
        </div>
        {!directsCollapsed && directs.length === 0 && (
          <div className="sb-item" style={{ color: "var(--fg-3)", cursor: "default" }}>
            <span className="sb-label">No direct messages yet</span>
          </div>
        )}
        {!directsCollapsed &&
          directs.map((d) => (
            <div
              key={d.id}
              className={
                "sb-item" +
                (selectedDmId === d.id ? " active" : "") +
                (d.unread ? " unread" : "")
              }
              onClick={() => onSelectDm(d)}
            >
              <span className="sb-icon">
                <span
                  style={{
                    width: 16,
                    height: 16,
                    borderRadius: "50%",
                    background: d.color,
                    color: "#fff",
                    fontSize: 9,
                    fontWeight: 700,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    position: "relative",
                  }}
                >
                  {d.initials}
                  {d.online && (
                    <span
                      style={{
                        position: "absolute",
                        right: -1,
                        bottom: -1,
                        width: 6,
                        height: 6,
                        borderRadius: "50%",
                        background: "var(--voice-live)",
                        boxShadow: "0 0 0 1.5px var(--surface-canvas)",
                      }}
                    />
                  )}
                </span>
              </span>
              <span className="sb-label">{d.name}</span>
              {d.unread > 0 && <span className="meta">{d.unread}</span>}
            </div>
          ))}

        {/* Servers */}
        {servers.length === 0 && (
          <div style={{ padding: "12px 10px", fontSize: 12, color: "var(--fg-3)" }}>
            No servers yet. Create one to get started.
          </div>
        )}

        {servers.map((server) => {
          const isCol = expandedServerId !== server.id;
          const channels = channelsByServer[server.id] || [];
          const accent = accentFor(server.id);
          return (
            <div key={server.id}>
              <div
                className={"sb-section-head" + (isCol ? " collapsed" : "")}
                onClick={() => {
                  onToggleServer(server.id);
                  onSelectServer(server.id);
                }}
                onContextMenu={(e) => {
                  e.preventDefault();
                  onDeleteServer(server);
                }}
                style={{ marginTop: 6 }}
              >
                <span
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                    minWidth: 0,
                    flex: 1,
                    overflow: "hidden",
                  }}
                >
                  <span style={{ color: accent, fontSize: 10, flexShrink: 0 }}>●</span>
                  <span
                    style={{
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {server.name}
                  </span>
                </span>
                <span style={{ display: "inline-flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
                  {!isCol && (
                    <button
                      type="button"
                      className="sb-section-add"
                      title="Create channel"
                      onClick={(e) => {
                        e.stopPropagation();
                        onAddChannel(server);
                      }}
                    >
                      <Icon name="plus" size={14} />
                    </button>
                  )}
                  <Icon name="chev" size={12} className="chev" />
                </span>
              </div>
              {!isCol && (
                <>
                  <div
                    className={
                      "sb-item" +
                      (selectedServerId === server.id && !selectedChannelId && !selectedDmId
                        ? " active"
                        : "")
                    }
                    onClick={() => onSelectServer(server.id)}
                  >
                    <span className="sb-icon">
                      <Icon name="hash" size={14} />
                    </span>
                    <span className="sb-label">general</span>
                  </div>
                  {channels.map((channel) => (
                    <div
                      key={channel.id}
                      className={
                        "sb-item" +
                        (selectedChannelId === channel.id ? " active" : "") +
                        (joinedChannelId === channel.id ? " in-voice" : "")
                      }
                      onClick={() => onSelectChannel(server, channel)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        onDeleteChannel(server, channel);
                      }}
                    >
                      <span className="sb-icon">
                        <Icon name="volume" size={14} />
                      </span>
                      <span className="sb-label">{channel.name}</span>
                      {channel.visibility === "private" && (
                        <span className="lock">
                          <Icon name="lock" size={11} />
                        </span>
                      )}
                      {joinedChannelId === channel.id && (
                        <span className="sb-live-dot" aria-hidden="true" />
                      )}
                    </div>
                  ))}
                </>
              )}
            </div>
          );
        })}

        <button type="button" className="sb-create-server" onClick={onAddServer}>
          <span className="plus">
            <Icon name="plus" size={12} />
          </span>
          <span>Add server</span>
        </button>
      </div>

      <div className="sb-footer">
        <div className="user-pill">
          <div className="ava">
            {currentUserName.charAt(0).toUpperCase()}
            <span className="dot" />
          </div>
          <div className="info">
            <div className="n">{currentUserName}</div>
            <div className="s">{currentUserStatus}</div>
          </div>
          <button type="button" className="icon-btn" title="Settings" onClick={onOpenSettings}>
            <Icon name="settings" size={14} />
          </button>
        </div>
      </div>
    </div>
  );
};

export default Sidebar;
