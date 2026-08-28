import React, { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";

import type { ActiveFriend, RecentRoom } from "../../api/home";
import type { Friendship } from "../../types/friendship";
import type { DmPreview } from "../../store/dmPreviewStore";
import { colorForId } from "../../utils/avatar";
import { UserAvatar } from "./Avatar";
import Icon from "./Icon";
import "../../styles/home.css";

export interface HomeViewProps {
  userName: string;
  currentUserId?: string;
  activeFriends: ActiveFriend[];
  recentRooms: RecentRoom[];
  friends: Friendship[];
  onlineUsers: Record<string, true>;
  unreadByDm: Record<string, number>;
  dmPreviews: Record<string, DmPreview>;
  joinedChannelId: string | null;
  onJoinRoom: (serverId: string, channelId: string) => void;
  onOpenDm: (friendId: string) => void;
  onCall: (friendId: string) => void;
  onAcceptRequest: (friendId: string) => void;
  onIgnoreRequest: (friendId: string) => void;
  onNewRoom: () => void;
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

const formatClock = (iso: string) => {
  const d = new Date(iso);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
};

const HomeView: React.FC<HomeViewProps> = ({
  userName,
  currentUserId,
  activeFriends,
  recentRooms,
  friends,
  onlineUsers,
  unreadByDm,
  dmPreviews,
  joinedChannelId,
  onJoinRoom,
  onOpenDm,
  onCall,
  onAcceptRequest,
  onIgnoreRequest,
  onNewRoom,
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
  const unread = useMemo(
    () =>
      accepted
        .filter((f) => (unreadByDm[f.friendId] ?? 0) > 0)
        .map((f) => ({ friend: f, preview: dmPreviews[f.friendId] }))
        .sort((a, b) => {
          const ta = a.preview ? new Date(a.preview.createdAt).getTime() : 0;
          const tb = b.preview ? new Date(b.preview.createdAt).getTime() : 0;
          return tb - ta;
        }),
    [accepted, unreadByDm, dmPreviews],
  );

  const previewText = (p: DmPreview | undefined) => {
    if (!p) return "";
    const body = p.callStatus
      ? t("previewCall")
      : p.inviteServerId
        ? t("previewInvite")
        : p.content || (p.hasAttachments ? t("previewAttachment") : "");
    return p.senderId === currentUserId ? `${t("previewYou")} ${body}` : body;
  };

  const onlineCount = onlineFriends.length + activeFriends.length;

  return (
    <div className="home">
      <header className="home-header">
        <span className="home-title">{t("title")}</span>
        <span className="home-sep" />
        <span className="home-sub">
          {t(greetingKey(new Date(now).getHours()))}, {userName}
        </span>
        <button type="button" className="home-btn-primary" onClick={onNewRoom}>
          <Icon name="plus" size={14} />
          {t("newRoom")}
        </button>
      </header>
      <div className="home-scroll">
        <div className="home-grid">
          <div className="home-col-main">
            <section>
              <h6 className="home-label">{t("activeNow", { count: activeFriends.length })}</h6>
              {activeFriends.length === 0 ? (
                <div className="home-empty">{t("emptyActive")}</div>
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
            </section>
            <section>
              <h6 className="home-label">{t("online", { count: onlineCount })}</h6>
              {onlineFriends.length === 0 && activeFriends.length === 0 ? (
                <div className="home-empty">{t("emptyOnline")}</div>
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
            {requests.length > 0 && (
              <div className="home-card home-side-card">
                <h6 className="home-label">{t("friendRequests", { count: requests.length })}</h6>
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
            )}
            <div className="home-card home-side-card">
              <h6 className="home-label">{t("jumpBackIn")}</h6>
              {recentRooms.length === 0 ? (
                <div className="home-empty">{t("emptyRooms")}</div>
              ) : (
                recentRooms.map((room) => (
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
                ))
              )}
            </div>
            <div className="home-card home-side-card">
              <h6 className="home-label">{t("unread")}</h6>
              {unread.length === 0 ? (
                <div className="home-empty">{t("emptyUnread")}</div>
              ) : (
                unread.map(({ friend, preview }) => (
                  <button
                    key={friend.friendId}
                    type="button"
                    className="home-unread-row"
                    onClick={() => onOpenDm(friend.friendId)}
                  >
                    <UserAvatar
                      userId={friend.friendId}
                      name={friend.friendName}
                      className="home-avatar"
                      style={{ background: colorForId(friend.friendId), width: 26, height: 26 }}
                    />
                    <span className="who">
                      <span className="line">
                        <span className="name">{friend.friendName}</span>
                        {preview && <span className="time">{formatClock(preview.createdAt)}</span>}
                      </span>
                      <span className="text">{previewText(preview)}</span>
                    </span>
                  </button>
                ))
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default HomeView;
