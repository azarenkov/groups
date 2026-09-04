import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";

import type { ActiveFriend, HomeOverview, RecentRoom } from "../../api/home";
import type { Friendship } from "../../types/friendship";
import { colorForId } from "../../utils/avatar";
import { useActivityStore } from "../../store/activityStore";
import { UserAvatar } from "./Avatar";
import Icon from "./Icon";
import "../../styles/home.css";

export interface HomeViewProps {
  userName: string;
  overview: HomeOverview | null;
  overviewFailed: boolean;
  onRetry: () => void;
  friends: Friendship[];
  onlineUsers: Record<string, true>;
  joinedChannelId: string | null;
  onJoinRoom: (serverId: string, channelId: string) => void;
  onOpenDm: (friendId: string) => void;
  onCall: (friendId: string) => void;
  onAddFriend: () => void;
}

const greetingKey = (hour: number) =>
  hour < 6
    ? "greetingNight"
    : hour < 12
      ? "greetingMorning"
      : hour < 18
        ? "greetingAfternoon"
        : "greetingEvening";

const useNow = (intervalMs: number) => {
  const [now, setNow] = React.useState(() => Date.now());
  useEffect(() => {
    const handle = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(handle);
  }, [intervalMs]);
  return now;
};

const HomeView: React.FC<HomeViewProps> = ({
  userName,
  overview,
  overviewFailed,
  onRetry,
  friends,
  onlineUsers,
  joinedChannelId,
  onJoinRoom,
  onOpenDm,
  onCall,
  onAddFriend,
}) => {
  const { t } = useTranslation("home");
  const now = useNow(30_000);
  const activities = useActivityStore((s) => s.byUser);
  const activeFriends = useMemo<ActiveFriend[]>(() => overview?.activeFriends ?? [], [overview]);
  const recentRooms = useMemo<RecentRoom[]>(() => overview?.recentRooms ?? [], [overview]);

  const duration = (joinedAt: string) => {
    const minutes = Math.max(0, Math.floor((now - new Date(joinedAt).getTime()) / 60_000));
    if (minutes < 60) return t("durationMinutes", { count: minutes });
    return t("durationHours", { hours: Math.floor(minutes / 60), minutes: minutes % 60 });
  };

  const activeIds = useMemo(
    () => new Set(activeFriends.map((f) => f.userId)),
    [activeFriends],
  );
  const accepted = useMemo(
    () => friends.filter((f) => f.status === "accepted"),
    [friends],
  );
  const onlineFriends = useMemo(
    () => accepted.filter((f) => !!onlineUsers[f.friendId] || activeIds.has(f.friendId)),
    [accepted, onlineUsers, activeIds],
  );
  const onlineCount = onlineFriends.length;
  const activeRooms = useMemo(() => {
    const rooms = new Map<string, { channelId: string; channelName: string; serverId: string; serverName: string; members: ActiveFriend[] }>();
    for (const f of activeFriends) {
      const room = rooms.get(f.channelId);
      if (room) room.members.push(f);
      else rooms.set(f.channelId, { channelId: f.channelId, channelName: f.channelName, serverId: f.serverId, serverName: f.serverName, members: [f] });
    }
    return Array.from(rooms.values());
  }, [activeFriends]);

  return (
    <div className="home">
      <header className="home-header">
        <span className="home-title">{t("title")}</span>
        <span className="home-sep" />
        <span className="home-sub">
          {t(greetingKey(new Date(now).getHours()))}, {userName}
        </span>
      </header>
      <div className="home-scroll">
        {!overview ? (
          <div className="home-state">
            {overviewFailed ? (
              <>
                <div className="home-empty-text">{t("loadFailed")}</div>
                <button type="button" className="home-btn-secondary sm" onClick={onRetry}>
                  <Icon name="refresh" size={13} />
                  {t("retry")}
                </button>
              </>
            ) : (
              <span className="loading-spinner" />
            )}
          </div>
        ) : (
        <div className="home-grid">
          <div className="home-col-main">
            <section>
              <h6 className="home-label">{t("online", { count: onlineCount })}</h6>
              {onlineFriends.length === 0 && activeFriends.length === 0 ? (
                <div className="home-empty home-card">
                  <span className="home-empty-ico"><Icon name="users" size={20} /></span>
                  <div className="home-empty-title">{t("emptyOnlineTitle")}</div>
                  <div className="home-empty-text">{t("emptyOnline")}</div>
                  <button type="button" className="home-btn-secondary sm" onClick={onAddFriend}>
                    <Icon name="user-plus" size={13} />
                    {t("addFriend")}
                  </button>
                </div>
              ) : (
                <div className="home-online-list">
                  {onlineFriends.map((f) => (
                    <div key={f.friendId} className="home-online-row">
                      <span className="home-avatar-wrap">
                        <UserAvatar
                          userId={f.friendId}
                          name={f.friendName}
                          className="home-avatar"
                          style={{ background: colorForId(f.friendId), width: 30, height: 30 }}
                        />
                        <span className="home-avatar-dot live" />
                      </span>
                      <span className="who">
                        <span className="name">{f.friendName}</span>
                        <span className="status">
                          {activities[f.friendId]
                            ? t("playingFor", { game: activities[f.friendId].name, duration: duration(activities[f.friendId].startedAt) })
                            : activeIds.has(f.friendId)
                              ? t("statusInVoice")
                              : t("statusOnline")}
                        </span>
                      </span>
                      <button
                        type="button"
                        className="home-icon-btn"
                        title={t("message")}
                        onClick={() => onOpenDm(f.friendId)}
                      >
                        <Icon name="message" size={15} />
                      </button>
                      <button
                        type="button"
                        className="home-icon-btn"
                        title={t("call")}
                        onClick={() => onCall(f.friendId)}
                      >
                        <Icon name="phone" size={15} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </section>
          </div>
          <div className="home-col-side">
            <div className="home-card home-side-card">
              <h6 className="home-label">{t("activeNow", { count: activeFriends.length })}</h6>
              {activeFriends.length === 0 ? (
                <div className="home-empty compact">
                  <span className="home-empty-ico"><Icon name="headphones" size={16} /></span>
                  <div className="home-empty-text">{t("emptyActive")}</div>
                </div>
              ) : (
                <div className="home-active-grid">
                  {activeRooms.map((room) => {
                    const connected = joinedChannelId === room.channelId;
                    return (
                      <div key={room.channelId} className="home-card home-active-card">
                        <div className="members">
                          {room.members.map((f) => (
                            <div key={f.userId} className="head">
                              <span className="home-avatar-wrap">
                                <UserAvatar
                                  userId={f.userId}
                                  name={f.userName}
                                  className="home-avatar"
                                  style={{ background: colorForId(f.userId), width: 32, height: 32 }}
                                />
                                <span className="home-avatar-dot live" />
                              </span>
                              <span className="who">
                                <span className="name">{f.userName}</span>
                                <span className="state">
                                  {f.isMuted && <Icon name="mic-off" size={11} />}
                                  {t("inVoice", { duration: duration(f.joinedAt) })}
                                </span>
                              </span>
                            </div>
                          ))}
                        </div>
                        <div className="foot">
                          <span className="room">
                            <Icon name="volume" size={13} />
                            <span>
                              {room.channelName} · {room.serverName}
                            </span>
                          </span>
                          <button
                            type="button"
                            className={connected ? "home-btn-joined" : "home-btn-secondary"}
                            onClick={() => !connected && onJoinRoom(room.serverId, room.channelId)}
                          >
                            {connected ? t("connected") : t("join")}
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
            <div className="home-card home-side-card">
              <h6 className="home-label">{t("jumpBackIn")}</h6>
              {recentRooms.length === 0 ? (
                <div className="home-empty compact">
                  <span className="home-empty-ico"><Icon name="volume" size={16} /></span>
                  <div className="home-empty-text">{t("emptyRooms")}</div>
                </div>
              ) : (
                <div className="home-room-list">
                {recentRooms.map((room) => (
                  <button
                    key={room.channelId}
                    type="button"
                    className="home-room-row"
                    onClick={() => onJoinRoom(room.serverId, room.channelId)}
                  >
                    <span className="ico">
                      <Icon name="volume" size={14} />
                    </span>
                    <span className="who">
                      <span className="name">{room.channelName}</span>
                      <span className="server">{room.serverName}</span>
                    </span>
                    {room.liveCount > 0 && (
                      <span className="home-live-pill">
                        <i />
                        {room.liveCount}
                      </span>
                    )}
                  </button>
                ))}
                </div>
              )}
            </div>
          </div>
        </div>
        )}
      </div>
    </div>
  );
};

export default HomeView;
