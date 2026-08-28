import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";

import type { ActiveFriend, RecentRoom } from "../../api/home";
import type { Friendship } from "../../types/friendship";
import { colorForId } from "../../utils/avatar";
import { UserAvatar } from "./Avatar";
import Icon from "./Icon";
import "../../styles/home.css";

export interface HomeViewProps {
  userName: string;
  activeFriends: ActiveFriend[];
  recentRooms: RecentRoom[];
  friends: Friendship[];
  onlineUsers: Record<string, true>;
  joinedChannelId: string | null;
  onJoinRoom: (serverId: string, channelId: string) => void;
  onOpenDm: (friendId: string) => void;
  onCall: (friendId: string) => void;
  onAcceptRequest: (friendId: string) => void;
  onIgnoreRequest: (friendId: string) => void;
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
  activeFriends,
  recentRooms,
  friends,
  onlineUsers,
  joinedChannelId,
  onJoinRoom,
  onOpenDm,
  onCall,
  onAcceptRequest,
  onIgnoreRequest,
  onAddFriend,
}) => {
  const { t } = useTranslation("home");
  const now = useNow(30_000);

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
    () =>
      accepted.filter((f) => !!onlineUsers[f.friendId] && !activeIds.has(f.friendId)),
    [accepted, onlineUsers, activeIds],
  );
  const requests = useMemo(
    () => friends.filter((f) => f.status === "pending_incoming"),
    [friends],
  );
  const onlineCount = onlineFriends.length + activeFriends.length;

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
                        <span className="status">{t("statusOnline")}</span>
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
                  {activeFriends.map((f) => {
                    const connected = joinedChannelId === f.channelId;
                    return (
                      <div key={`${f.userId}:${f.channelId}`} className="home-card home-active-card">
                        <div className="head">
                          <span className="home-avatar-wrap">
                            <UserAvatar
                              userId={f.userId}
                              name={f.userName}
                              className="home-avatar"
                              style={{ background: colorForId(f.userId), width: 36, height: 36 }}
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
                        <div className="foot">
                          <span className="room">
                            <Icon name="volume" size={13} />
                            <span>
                              {f.channelName} · {f.serverName}
                            </span>
                          </span>
                          <button
                            type="button"
                            className={connected ? "home-btn-joined" : "home-btn-secondary"}
                            onClick={() => !connected && onJoinRoom(f.serverId, f.channelId)}
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
          {requests.length > 0 && (
              <div className="home-card home-side-card">
                <h6 className="home-label">{t("friendRequests", { count: requests.length })}</h6>
                <div className="home-request-list">
                {requests.map((r) => (
                  <div key={r.friendId} className="home-request-row">
                    <UserAvatar
                      userId={r.friendId}
                      name={r.friendName}
                      className="home-avatar"
                      style={{ background: colorForId(r.friendId), width: 30, height: 30 }}
                    />
                    <span className="who">
                      <span className="name">{r.friendName}</span>
                    </span>
                    <div className="actions">
                      <button
                        type="button"
                        className="home-btn-primary sm"
                        onClick={() => onAcceptRequest(r.friendId)}
                      >
                        {t("accept")}
                      </button>
                      <button
                        type="button"
                        className="home-btn-secondary sm"
                        onClick={() => onIgnoreRequest(r.friendId)}
                      >
                        {t("ignore")}
                      </button>
                    </div>
                  </div>
                ))}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default HomeView;
