import React from "react";
import Icon from "./Icon";
import type { Server } from "../../types/server";
import type { Channel } from "../../types/channel";
import type { MockDirect } from "../../data/mockData";

interface SidebarProps {
  servers: Server[];
  channelsByServer: Record<string, Channel[]>;
  expandedServerIds: Set<string>;
  selectedChannelId: string | null;
  joinedChannelId: string | null;
  selectedDmId: string | null;
  directs: MockDirect[];
  pendingCount?: number;
  onToggleServer: (serverId: string) => void;
  onSelectServer: (serverId: string) => void;
  onSelectChannel: (server: Server, channel: Channel) => void;
  onSelectDm: (dm: MockDirect) => void;
  onAddServer: () => void;
  onAddChannel: (server: Server) => void;
  onInviteServer: (server: Server) => void;
  onAddDm: () => void;
  onOpenPendingRequests?: () => void;
  onDeleteServer: (server: Server) => void;
  onDeleteChannel: (server: Server, channel: Channel) => void;
  onOpenSettings: () => void;
  currentUserId?: string;
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
  expandedServerIds,
  selectedChannelId,
  joinedChannelId,
  selectedDmId,
  directs,
  pendingCount = 0,
  onToggleServer,
  onSelectServer,
  onSelectChannel,
  onSelectDm,
  onAddServer,
  onAddChannel,
  onInviteServer,
  onAddDm,
  onOpenPendingRequests,
  onDeleteServer,
  onDeleteChannel,
  onOpenSettings,
  currentUserId,
  currentUserName,
  currentUserStatus,
}) => {
  const [collapsed, setCollapsed] = React.useState<Record<string, boolean>>({});
  const toggleSection = (id: string) =>
    setCollapsed((c) => ({ ...c, [id]: !c[id] }));
  const [menuServerId, setMenuServerId] = React.useState<string | null>(null);
  const menuRef = React.useRef<HTMLDivElement>(null);

  React.useEffect(() => {
    if (!menuServerId) return;
    const onDoc = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuServerId(null);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMenuServerId(null);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [menuServerId]);

  const directsCollapsed = !!collapsed["__directs"];

  const searchInputRef = React.useRef<HTMLInputElement>(null);
  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="sb">
      <div className="sb-search">
        <Icon name="search" size={14} />
        <input ref={searchInputRef} placeholder="Search" />
        <kbd>⌘K</kbd>
      </div>

      <div className="sb-scroll">
        <div
          className={"sb-section-head" + (directsCollapsed ? " collapsed" : "")}
          onClick={() => toggleSection("__directs")}
        >
          <span>Direct Messages</span>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            {onOpenPendingRequests && (
              <button
                type="button"
                tabIndex={-1}
                className="sb-section-add sb-pending-toggle"
                title="Friend requests"
                onClick={(e) => {
                  e.stopPropagation();
                  onOpenPendingRequests();
                }}
              >
                <Icon name="bell" size={14} />
                {pendingCount > 0 && (
                  <span className="sb-pending-badge">{pendingCount}</span>
                )}
              </button>
            )}
            <button
              type="button"
              tabIndex={-1}
              className="sb-section-add"
              title="Add friend"
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
              role="button"
              tabIndex={0}
              className={
                "sb-item" +
                (selectedDmId === d.id ? " active" : "") +
                (d.unread ? " unread" : "")
              }
              onClick={() => onSelectDm(d)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onSelectDm(d);
                }
              }}
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

        {servers.length === 0 && (
          <div style={{ padding: "12px 10px", fontSize: 12, color: "var(--fg-3)" }}>
            No servers yet. Create one to get started.
          </div>
        )}

        {servers.map((server) => {
          const isCol = !expandedServerIds.has(server.id);
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
                    <>
                      <button
                        type="button"
                        tabIndex={-1}
                        className="sb-section-add"
                        title="Invite people"
                        onClick={(e) => {
                          e.stopPropagation();
                          onInviteServer(server);
                        }}
                      >
                        <Icon name="user-plus" size={14} />
                      </button>
                      <div
                        className="sb-server-menu-wrap"
                        ref={menuServerId === server.id ? menuRef : undefined}
                      >
                        <button
                          type="button"
                          tabIndex={-1}
                          className={
                            "sb-section-add" +
                            (menuServerId === server.id ? " on" : "")
                          }
                          title="More"
                          onClick={(e) => {
                            e.stopPropagation();
                            setMenuServerId((prev) =>
                              prev === server.id ? null : server.id,
                            );
                          }}
                        >
                          <Icon name="more" size={14} />
                        </button>
                        {menuServerId === server.id && (
                          <div
                            className="sb-server-menu"
                            onClick={(e) => e.stopPropagation()}
                          >
                            <button
                              type="button"
                              className="sb-server-menu-item"
                              onClick={() => {
                                setMenuServerId(null);
                                onAddChannel(server);
                              }}
                            >
                              <Icon name="plus" size={12} />
                              <span>Create channel</span>
                            </button>
                            <div className="sb-server-menu-divider" />
                            <button
                              type="button"
                              className="sb-server-menu-item danger"
                              onClick={() => {
                                setMenuServerId(null);
                                onDeleteServer(server);
                              }}
                            >
                              <Icon
                                name={
                                  currentUserId === server.ownerId
                                    ? "trash"
                                    : "logout"
                                }
                                size={12}
                              />
                              <span>
                                {currentUserId === server.ownerId
                                  ? "Delete server"
                                  : "Leave server"}
                              </span>
                            </button>
                          </div>
                        )}
                      </div>
                    </>
                  )}
                  <Icon name="chev" size={12} className="chev" />
                </span>
              </div>
              {!isCol && (
                <>
                  {channels.map((channel) => (
                    <div
                      key={channel.id}
                      role="button"
                      tabIndex={0}
                      className={
                        "sb-item" +
                        (selectedChannelId === channel.id ? " active" : "") +
                        (joinedChannelId === channel.id ? " in-voice" : "")
                      }
                      onClick={() => onSelectChannel(server, channel)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          onSelectChannel(server, channel);
                        }
                      }}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        onDeleteChannel(server, channel);
                      }}
                    >
                      <span className="sb-icon">
                        <Icon
                          name={channel.kind === "voice" ? "volume" : "hash"}
                          size={14}
                        />
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

        <button type="button" tabIndex={-1} className="sb-create-server" onClick={onAddServer}>
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
