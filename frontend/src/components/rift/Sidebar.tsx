import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import type { Server } from "../../types/server";
import type { Channel } from "../../types/channel";
import type { MockDirect } from "../../data/mockData";
import { colorForId } from "../../utils/avatar";

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
  voiceMembers?: {
    id: string;
    name: string;
    initials: string;
    color: string;
    isMuted?: boolean;
    isSharingScreen?: boolean;
  }[];
  voiceMembersByChannel?: Record<
    string,
    {
      id: string;
      name: string;
      initials: string;
      color: string;
      isMuted?: boolean;
      isSharingScreen?: boolean;
    }[]
  >;
  searchQuery: string;
  onSearchQueryChange: (value: string) => void;
  unreadByChannel?: Record<string, number>;
  unreadByDm?: Record<string, number>;
  channelToServer?: Record<string, string>;
  onSetStatus?: (status: "online" | "offline") => void;
}

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
  voiceMembers,
  voiceMembersByChannel,
  searchQuery,
  onSearchQueryChange,
  unreadByChannel = {},
  unreadByDm = {},
  channelToServer = {},
  onSetStatus,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
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

  const [statusMenuOpen, setStatusMenuOpen] = React.useState(false);
  const statusMenuRef = React.useRef<HTMLDivElement>(null);
  React.useEffect(() => {
    if (!statusMenuOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (
        statusMenuRef.current &&
        !statusMenuRef.current.contains(e.target as Node)
      ) {
        setStatusMenuOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setStatusMenuOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [statusMenuOpen]);

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

  const query = searchQuery;
  const setQuery = onSearchQueryChange;
  const q = query.trim().toLowerCase();
  const match = (s: string) => s.toLowerCase().includes(q);
  const isFiltering = q.length > 0;

  const visibleDirects = isFiltering
    ? directs.filter((d) => match(d.name))
    : directs;

  const filteredServers = isFiltering
    ? servers.filter((server) => {
        if (match(server.name)) return true;
        const channels = channelsByServer[server.id] || [];
        return channels.some((c) => match(c.name));
      })
    : servers;

  const visibleChannelsFor = (serverId: string) => {
    const channels = channelsByServer[serverId] || [];
    if (!isFiltering) return channels;
    const server = servers.find((s) => s.id === serverId);
    if (server && match(server.name)) return channels;
    return channels.filter((c) => match(c.name));
  };

  return (
    <div className="sb">
      <div className="sb-search">
        <Icon name="search" size={14} />
        <input
          ref={searchInputRef}
          placeholder={t("searchPlaceholder")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              setQuery("");
              searchInputRef.current?.blur();
            }
          }}
        />
        {query ? (
          <button
            type="button"
            tabIndex={-1}
            className="sb-search-clear"
            title={t("close", { ns: "common" })}
            onClick={() => {
              setQuery("");
              searchInputRef.current?.focus();
            }}
          >
            <Icon name="x" size={11} />
          </button>
        ) : (
          <kbd>⌘K</kbd>
        )}
      </div>

      <div className="sb-scroll">
        <div
          className={"sb-section-head" + (directsCollapsed ? " collapsed" : "")}
          onClick={() => toggleSection("__directs")}
        >
          <span>{t("directMessages")}</span>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            {onOpenPendingRequests && (
              <button
                type="button"
                tabIndex={-1}
                className="sb-section-add sb-pending-toggle"
                title={t("pendingRequests")}
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
              title={t("addDm")}
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
        {!directsCollapsed && visibleDirects.length === 0 && (
          <div className="sb-item" style={{ color: "var(--fg-3)", cursor: "default" }}>
            <span className="sb-label">
              {isFiltering ? "No matches" : "No direct messages yet"}
            </span>
          </div>
        )}
        {!directsCollapsed &&
          visibleDirects.map((d) => (
            <div
              key={d.id}
              role="button"
              tabIndex={0}
              className={
                "sb-item" +
                (selectedDmId === d.id ? " active" : "") +
                ((unreadByDm[d.id] ?? 0) > 0 ? " unread" : "")
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
              {(unreadByDm[d.id] ?? 0) > 0 && (
                <span className="meta">{unreadByDm[d.id]}</span>
              )}
            </div>
          ))}

        <div
          className="sb-section-head static"
          style={{ padding: "4px 8px 0", cursor: "default" }}
        >
          <span>{t("servers")}</span>
        </div>

        {filteredServers.length === 0 && (
          <div style={{ padding: "12px 10px", fontSize: 12, color: "var(--fg-3)" }}>
            {isFiltering ? t("noMatches") : t("noServers")}
          </div>
        )}

        {filteredServers.map((server) => {
          const isCol = isFiltering ? false : !expandedServerIds.has(server.id);
          const channels = visibleChannelsFor(server.id);
          const accent = server.accentColor ?? colorForId(server.id);
          const allServerChannels = channelsByServer[server.id] || [];
          const localSum = allServerChannels
            .filter((c) => c.kind === "text")
            .reduce((sum, c) => sum + (unreadByChannel[c.id] ?? 0), 0);
          let fallbackSum = 0;
          for (const [cid, count] of Object.entries(unreadByChannel)) {
            if (channelToServer[cid] === server.id) fallbackSum += count;
          }
          const serverUnread = Math.max(localSum, fallbackSum);
          const hasVoiceActivity = allServerChannels.some(
            (c) =>
              c.kind === "voice" &&
              (voiceMembersByChannel?.[c.id]?.length ?? 0) > 0,
          );
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
                  {hasVoiceActivity && (
                    <span
                      className="sb-server-voice-dot"
                      title="Someone is in voice"
                      aria-label="Voice activity"
                    />
                  )}
                </span>
                <span style={{ display: "inline-flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
                  {serverUnread > 0 && (
                    <span className="sb-server-unread">{serverUnread}</span>
                  )}
                  {!isCol && (
                    <>
                      <button
                        type="button"
                        tabIndex={-1}
                        className="sb-section-add"
                        title={t("inviteFriends")}
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
                              <span>{t("addChannel")}</span>
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
                                  ? t("deleteServer")
                                  : t("leaveServer")}
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
                    <React.Fragment key={channel.id}>
                      <div
                        role="button"
                        tabIndex={0}
                        className={
                          "sb-item" +
                          (selectedChannelId === channel.id ? " active" : "") +
                          (joinedChannelId === channel.id ? " in-voice" : "") +
                          (channel.kind === "text" &&
                          (unreadByChannel[channel.id] ?? 0) > 0
                            ? " unread"
                            : "")
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
                        {channel.kind === "text" &&
                          (unreadByChannel[channel.id] ?? 0) > 0 && (
                            <span className="meta">
                              {unreadByChannel[channel.id]}
                            </span>
                          )}
                        {channel.kind === "voice" &&
                          (voiceMembersByChannel?.[channel.id]?.length ?? 0) >
                            0 && (
                            <span className="meta">
                              {voiceMembersByChannel?.[channel.id]?.length}
                            </span>
                          )}
                        {channel.visibility === "private" && (
                          <span className="lock">
                            <Icon name="lock" size={11} />
                          </span>
                        )}
                      </div>
                      {channel.kind === "voice" &&
                        (() => {
                          const base = voiceMembersByChannel?.[channel.id] ?? [];
                          const isJoined = joinedChannelId === channel.id;
                          let membersForChannel = base;
                          if (isJoined && voiceMembers && voiceMembers.length > 0) {
                            const merged = new Map(base.map((m) => [m.id, m]));
                            for (const live of voiceMembers) {
                              const existing = merged.get(live.id);
                              merged.set(
                                live.id,
                                existing ? { ...existing, ...live } : live,
                              );
                            }
                            membersForChannel = Array.from(merged.values());
                          }
                          if (membersForChannel.length === 0) return null;
                          return (
                            <div className="sb-voice-members">
                              {membersForChannel.map((m) => (
                              <div key={m.id} className="sb-voice-member">
                                <span
                                  className="sb-voice-member-ava"
                                  style={{ background: m.color }}
                                >
                                  {m.initials}
                                </span>
                                <span className="sb-voice-member-name">
                                  {m.name}
                                </span>
                                {m.isSharingScreen && (
                                  <span
                                    className="sb-voice-member-badge"
                                    title="Sharing screen"
                                  >
                                    <Icon name="monitor" size={11} />
                                  </span>
                                )}
                                {m.isMuted && (
                                  <span
                                    className="sb-voice-member-badge muted"
                                    title="Muted"
                                  >
                                    <Icon name="mic-off" size={11} />
                                  </span>
                                )}
                              </div>
                            ))}
                            </div>
                          );
                        })()}
                    </React.Fragment>
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
          <span>{t("addServer")}</span>
        </button>
      </div>

      <div id="voice-dock-slot" className="sb-voice-dock" />

      <div className="sb-footer">
        <div className="user-pill">
          <div
            className="ava"
            style={
              currentUserId
                ? { background: colorForId(currentUserId) }
                : undefined
            }
          >
            {currentUserName.charAt(0).toUpperCase()}
            <span
              className={
                "dot" + (currentUserStatus === "offline" ? " offline" : "")
              }
            />
          </div>
          <div
            className={
              "info user-pill-info-btn" +
              (statusMenuOpen ? " open" : "")
            }
            ref={statusMenuRef}
            role="button"
            tabIndex={0}
            title={t("changeStatus")}
            onClick={() => setStatusMenuOpen((v) => !v)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                setStatusMenuOpen((v) => !v);
              }
            }}
          >
            <div className="n">{currentUserName}</div>
            <div className="s">
              <span>
                {currentUserStatus === "offline"
                  ? t("statusOffline")
                  : t("statusOnline")}
              </span>
              <Icon
                name="chev"
                size={9}
                className={"user-pill-chev" + (statusMenuOpen ? " up" : "")}
              />
            </div>
            {statusMenuOpen && onSetStatus && (
              <div className="sb-status-menu" onClick={(e) => e.stopPropagation()}>
                <button
                  type="button"
                  className="sb-status-menu-item"
                  onClick={() => {
                    onSetStatus("online");
                    setStatusMenuOpen(false);
                  }}
                >
                  <span className="sb-status-dot online" />
                  <span>{t("statusOnline")}</span>
                </button>
                <button
                  type="button"
                  className="sb-status-menu-item"
                  onClick={() => {
                    onSetStatus("offline");
                    setStatusMenuOpen(false);
                  }}
                >
                  <span className="sb-status-dot offline" />
                  <span>{t("statusOffline")}</span>
                </button>
              </div>
            )}
          </div>
          <button type="button" className="icon-btn" title={t("settings")} onClick={onOpenSettings}>
            <Icon name="settings" size={14} />
          </button>
        </div>
      </div>
    </div>
  );
};

export default Sidebar;
