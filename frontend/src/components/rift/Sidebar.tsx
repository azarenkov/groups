import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import type { Server } from "../../types/server";
import type { Channel } from "../../types/channel";
import type { MockDirect } from "../../data/mockData";
import { colorForId } from "../../utils/avatar";
import { ServerLogo, UserAvatar } from "./Avatar";

interface SidebarProps {
  servers: Server[];
  channelsByServer: Record<string, Channel[]>;
  expandedServerIds: Set<string>;
  selectedChannelId: string | null;
  joinedChannelId: string | null;
  selectedDmId: string | null;
  directs: MockDirect[];
  onToggleServer: (serverId: string) => void;
  onSelectServer: (serverId: string) => void;
  onSelectChannel: (server: Server, channel: Channel) => void;
  onSelectDm: (dm: MockDirect) => void;
  onAddChannel: (server: Server) => void;
  onInviteServer: (server: Server) => void;
  onOpenServerSettings: (server: Server) => void;
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
    isCameraOn?: boolean;
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
      isCameraOn?: boolean;
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
  onToggleServer,
  onSelectServer,
  onSelectChannel,
  onSelectDm,
  onAddChannel,
  onInviteServer,
  onOpenServerSettings,
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
  const [tab, setTab] = React.useState<"dms" | "servers">(
    selectedChannelId ? "servers" : "dms",
  );
  const prevSelection = React.useRef({
    dm: selectedDmId,
    channel: selectedChannelId,
  });
  React.useEffect(() => {
    const prev = prevSelection.current;
    if (selectedDmId && selectedDmId !== prev.dm) {
      setTab("dms");
    } else if (selectedChannelId && selectedChannelId !== prev.channel) {
      setTab("servers");
    }
    prevSelection.current = { dm: selectedDmId, channel: selectedChannelId };
  }, [selectedDmId, selectedChannelId]);
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

  const settleAfterPick = (target: "dms" | "servers") => {
    if (!isFiltering) return;
    setTab(target);
    setQuery("");
  };

  const pickDm = (d: MockDirect) => {
    onSelectDm(d);
    settleAfterPick("dms");
  };

  const pickServer = (server: Server) => {
    if (!isFiltering || !expandedServerIds.has(server.id)) {
      onToggleServer(server.id);
    }
    onSelectServer(server.id);
    settleAfterPick("servers");
  };

  const pickChannel = (server: Server, channel: Channel) => {
    onSelectChannel(server, channel);
    if (isFiltering && !expandedServerIds.has(server.id)) {
      onToggleServer(server.id);
    }
    settleAfterPick("servers");
  };

  const renderDms = () => (
    <div className="sb-view">
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("directMessages")}</span>
      </div>
      {visibleDirects.length === 0 && (
        <div className="sb-empty">
          {isFiltering ? t("noDirectMatches") : t("noDirects")}
        </div>
      )}
      {visibleDirects.map((d) => {
        const unread = unreadByDm[d.id] ?? 0;
        const sub =
          d.lastMessage ||
          (d.online ? t("statusOnline") : t("statusOffline"));
        return (
          <div
            key={d.id}
            role="button"
            tabIndex={0}
            className={
              "sb-dm" +
              (selectedDmId === d.id ? " active" : "") +
              (!d.online ? " off" : "")
            }
            onClick={() => pickDm(d)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                pickDm(d);
              }
            }}
          >
            <div className="sb-dm-ava-wrap">
              <UserAvatar
                userId={d.id}
                name={d.name}
                className="sb-dm-ava"
                style={{ background: d.color }}
              />
              {d.online && <span className="sb-dm-dot" />}
            </div>
            <div className="sb-dm-main">
              <div className="sb-dm-name">{d.name}</div>
              <div className={"sb-dm-sub" + (unread > 0 ? " unread" : "")}>
                {sub}
              </div>
            </div>
            {unread > 0 && <span className="sb-dm-badge">{unread}</span>}
          </div>
        );
      })}
    </div>
  );

  const renderServers = () => (
    <div className="sb-view">
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("servers")}</span>
      </div>
      {filteredServers.length === 0 && (
        <div className="sb-empty">
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
              role="button"
              tabIndex={0}
              className="sb-srv"
              onClick={() => pickServer(server)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  pickServer(server);
                }
              }}
            >
              <ServerLogo
                serverId={server.id}
                name={server.name}
                className="sb-srv-ava"
                style={{ background: accent }}
              />
              <span className="sb-srv-name">{server.name}</span>
              {hasVoiceActivity && (
                <span
                  className="sb-server-voice-dot"
                  title="Someone is in voice"
                  aria-label="Voice activity"
                />
              )}
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
                        <button
                          type="button"
                          className="sb-server-menu-item"
                          onClick={() => {
                            setMenuServerId(null);
                            onOpenServerSettings(server);
                          }}
                        >
                          <Icon name="settings" size={12} />
                          <span>{t("serverSettings")}</span>
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
              <Icon
                name="chev"
                size={13}
                className={"sb-srv-chev" + (isCol ? " collapsed" : "")}
              />
            </div>
            {!isCol && (
              <div className="sb-srv-channels">
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
                      onClick={() => pickChannel(server, channel)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          pickChannel(server, channel);
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
                                <UserAvatar
                                  userId={m.id}
                                  name={m.name}
                                  className="sb-voice-member-ava"
                                  style={{ background: m.color, display: "inline-flex" }}
                                />
                                <span className="sb-voice-member-name">
                                  {m.name}
                                </span>
                                {m.isCameraOn && (
                                  <span
                                    className="sb-voice-member-badge"
                                    title="Camera on"
                                  >
                                    <Icon name="camera" size={11} />
                                  </span>
                                )}
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
              </div>
            )}
          </div>
        );
      })}
    </div>
  );

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

      <div className={"sb-seg " + tab}>
        <div className="sb-seg-thumb" />
        <button
          type="button"
          className={"sb-seg-btn" + (tab === "dms" ? " active" : "")}
          onClick={() => setTab("dms")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M7.9 20A9 9 0 1 0 4 16.1L2 22Z" />
          </svg>
          {t("tabDirects")}
        </button>
        <button
          type="button"
          className={"sb-seg-btn" + (tab === "servers" ? " active" : "")}
          onClick={() => setTab("servers")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <rect x="2" y="2" width="20" height="8" rx="2" />
            <rect x="2" y="14" width="20" height="8" rx="2" />
            <path d="M6 6h.01M6 18h.01" />
          </svg>
          {t("tabServers")}
        </button>
      </div>

      <div className="sb-scroll">
        {isFiltering ? (
          <>
            {renderDms()}
            {renderServers()}
          </>
        ) : tab === "dms" ? (
          renderDms()
        ) : (
          renderServers()
        )}
      </div>

      <div id="voice-dock-slot" className="sb-voice-dock" />

      <div className="sb-footer">
        <div className="user-pill">
          <UserAvatar
            userId={currentUserId}
            name={currentUserName}
            className="ava"
          >
            <span
              className={
                "dot" + (currentUserStatus === "offline" ? " offline" : "")
              }
            />
          </UserAvatar>
          <div
            className={"info user-pill-info-btn" + (statusMenuOpen ? " open" : "")}
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
              <div
                className="sb-status-menu"
                onClick={(e) => e.stopPropagation()}
              >
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
          <button
            type="button"
            className="icon-btn"
            title={t("settings")}
            onClick={onOpenSettings}
          >
            <Icon name="settings" size={14} />
          </button>
        </div>
      </div>
    </div>
  );
};

export default Sidebar;
