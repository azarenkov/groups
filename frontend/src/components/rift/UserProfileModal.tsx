import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "./Icon";
import { ServerLogo, UserAvatar } from "./Avatar";
import { homeAPI, type PublicProfile } from "../../api/home";
import { useAuthStore } from "../../store/authStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { usePresenceStore } from "../../store/presenceStore";
import { useActivityStore } from "../../store/activityStore";
import { useProfileModalStore } from "../../store/profileModalStore";
import { userAPI, type Connection } from "../../api/user";
import { useUserNamesStore } from "../../store/userNamesStore";
import { profileVersion, useUserProfileStore } from "../../store/userProfileStore";
import { colorForId } from "../../utils/avatar";
import { currentLocale } from "../../i18n";

interface Props {
  onOpenDm: (userId: string, name?: string) => void;
  onCall: (userId: string) => void;
  onRemoveFriend: (userId: string, name: string) => void;
}

const useMinuteTick = () => {
  const [, force] = React.useReducer((n: number) => n + 1, 0);
  React.useEffect(() => {
    const align = 60_000 - (Date.now() % 60_000);
    let interval: ReturnType<typeof setInterval> | null = null;
    const timeout = setTimeout(() => {
      force();
      interval = setInterval(force, 60_000);
    }, align);
    return () => {
      clearTimeout(timeout);
      if (interval) clearInterval(interval);
    };
  }, []);
};

const formatLocalTime = (timezone: string, locale: string): string | null => {
  if (!timezone) return null;
  try {
    const now = new Date();
    const time = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit", timeZone: timezone }).format(now);
    const parts = new Intl.DateTimeFormat("en-US", { timeZone: timezone, timeZoneName: "shortOffset" }).formatToParts(now);
    const offset = parts.find((p) => p.type === "timeZoneName")?.value ?? "";
    return offset ? `${time} (${offset})` : time;
  } catch {
    return null;
  }
};

const formatMemberSince = (iso: string | null, locale: string): string | null => {
  if (!iso) return null;
  const d = new Date(iso);
  if (!Number.isFinite(d.getTime())) return null;
  return new Intl.DateTimeFormat(locale, { day: "numeric", month: "short", year: "numeric" }).format(d);
};

const UserProfileModal: React.FC<Props> = ({ onOpenDm, onCall, onRemoveFriend }) => {
  const { t } = useTranslation("profile");
  const userId = useProfileModalStore((s) => s.userId);
  const close = useProfileModalStore((s) => s.close);
  const selfId = useAuthStore((s) => s.user?.id);
  const friends = useFriendshipStore((s) => s.friends);
  const onlineUsers = usePresenceStore((s) => s.online);
  const selfOnline = usePresenceStore((s) => s.selfOnline);
  const gameActivity = useActivityStore((s) => (userId ? s.byUser[userId] : undefined));
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const cachedBio = useUserProfileStore((s) => (userId ? s.bios[userId]?.bio : undefined));
  const [profile, setProfile] = React.useState<PublicProfile | null>(null);
  const [connections, setConnections] = React.useState<Connection[]>([]);
  const [failed, setFailed] = React.useState(false);
  const [menuOpen, setMenuOpen] = React.useState(false);
  const [copied, setCopied] = React.useState(false);
  const menuRef = React.useRef<HTMLDivElement>(null);
  useMinuteTick();

  const friendshipStatus = userId
    ? friends.find((f) => f.friendId === userId)?.status
    : undefined;

  React.useEffect(() => {
    setProfile(null);
    setConnections([]);
    setFailed(false);
    setMenuOpen(false);
    setCopied(false);
  }, [userId]);

  React.useEffect(() => {
    if (!userId) return;
    let cancelled = false;
    userAPI
      .userConnections(userId)
      .then((items) => {
        if (!cancelled) setConnections(items);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [userId]);

  React.useEffect(() => {
    if (!userId) return;
    let cancelled = false;
    homeAPI
      .profile(userId)
      .then((p) => {
        if (cancelled) return;
        useUserProfileStore.getState().apply(userId, p.bio, profileVersion(p.profileUpdatedAt));
        setProfile(p);
        setFailed(false);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [userId, friendshipStatus]);

  React.useEffect(() => {
    if (!userId) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        close();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [userId, close]);

  React.useEffect(() => {
    if (!menuOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setMenuOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [menuOpen]);

  if (!userId) return null;

  const friendship = friends.find((f) => f.friendId === userId);
  const isSelf = userId === selfId;
  const isFriend = friendship?.status === "accepted";
  const online = isSelf ? selfOnline : !!onlineUsers[userId];
  const name = nameOverrides[userId] ?? profile?.name ?? "";
  const bio = cachedBio ?? profile?.bio ?? "";
  const locale = currentLocale();
  const localTime = profile ? formatLocalTime(profile.timezone, locale) : null;
  const memberSince = profile ? formatMemberSince(profile.memberSince, locale) : null;
  const inactive = !!profile && !profile.active;
  const canMessage = !isSelf && !inactive && (profile ? profile.acceptsMessages : isFriend);
  const canCall = !isSelf && !inactive && (profile ? profile.acceptsCalls : isFriend);

  const copyLogin = async () => {
    if (!name) return;
    let ok = false;
    try {
      await navigator.clipboard.writeText(name);
      ok = true;
    } catch {
      const area = document.createElement("textarea");
      area.value = name;
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.select();
      try {
        ok = document.execCommand("copy");
      } catch {
        ok = false;
      }
      area.remove();
    }
    if (ok) {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }
    setMenuOpen(false);
  };

  return (
    <div className="modal-overlay profile-overlay" onMouseDown={close}>
      <div className="profile-window" onMouseDown={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div className="profile-band" style={{ background: `color-mix(in srgb, ${colorForId(userId)} 28%, var(--surface-raised))` }}>
          <button type="button" className="profile-close" onClick={close} aria-label={t("close")}>
            <Icon name="x" size={15} />
          </button>
        </div>
        <div className="profile-body">
          <div className="profile-avatar-row">
            <span className="profile-avatar-wrap">
              <UserAvatar userId={userId} name={name || "?"} className="profile-avatar" style={{ background: colorForId(userId) }} />
              <span className={"profile-presence " + (online ? "online" : "offline")} />
            </span>
            {profile?.voiceRoom && (
              <span className="profile-voice-pill" title={profile.voiceRoom.serverName}>
                <Icon name="volume" size={13} />
                {t("inVoice", { room: profile.voiceRoom.channelName })}
              </span>
            )}
          </div>

          {failed ? (
            <div className="profile-name">{t("unavailable")}</div>
          ) : (
            <>
              <div className="profile-name">{name || (inactive ? t("deletedUser") : "…")}</div>
              <div className="profile-handle">
                {name && !inactive ? `@${name} · ` : ""}
                {inactive
                  ? t("inactive")
                  : online && gameActivity
                    ? t("playing", { game: gameActivity.name })
                    : online
                      ? t("online")
                      : t("offline")}
              </div>
            </>
          )}

          {!isSelf && !inactive && !failed && (
            <div className="profile-actions">
              {!isFriend && (
                friendship?.status === "pending_outgoing" ? (
                  <button type="button" className="profile-btn icon" title={t("requestSent")} disabled>
                    <Icon name="check" size={22} />
                  </button>
                ) : friendship?.status === "pending_incoming" ? (
                  <button
                    type="button"
                    className="profile-btn icon primary"
                    title={t("acceptRequest")}
                    onClick={() => void useFriendshipStore.getState().accept(userId).catch(() => {})}
                  >
                    <Icon name="check" size={22} />
                  </button>
                ) : (
                  <button
                    type="button"
                    className="profile-btn icon primary"
                    title={profile && !profile.acceptsFriendRequests ? t("requestsClosed") : t("addFriend")}
                    disabled={!name || !profile || !profile.acceptsFriendRequests}
                    onClick={() => {
                      if (name) void useFriendshipStore.getState().sendRequest(name).catch(() => {});
                    }}
                  >
                    <Icon name="user-plus" size={22} />
                  </button>
                )
              )}
              <button type="button" className={"profile-btn" + (isFriend ? " primary" : "")} disabled={!canMessage} onClick={() => { close(); onOpenDm(userId, name || undefined); }}>
                <Icon name="message" size={15} />
                {t("message")}
              </button>
              <button type="button" className="profile-btn" disabled={!canCall} onClick={() => { close(); onCall(userId); }}>
                <Icon name="phone" size={15} />
                {t("call")}
              </button>
              <div className="profile-more" ref={menuRef}>
                <button type="button" className="profile-btn icon" aria-label={t("more")} onClick={() => setMenuOpen((v) => !v)}>
                  <Icon name="more" size={16} />
                </button>
                {menuOpen && (
                  <div className="profile-menu">
                    {isFriend && (
                      <button type="button" className="danger" onClick={() => { setMenuOpen(false); close(); onRemoveFriend(userId, name); }}>
                        {t("removeFriend")}
                      </button>
                    )}
                    <button type="button" onClick={() => void copyLogin()} disabled={!name}>{copied ? t("copied") : t("copyLogin")}</button>
                  </div>
                )}
              </div>
            </div>
          )}

          {!failed && (
            <>
              <div className="profile-divider" />
              {bio && (
                <>
                  <div className="profile-label">{t("about")}</div>
                  <div className="profile-about">{bio}</div>
                </>
              )}
              <div className={"profile-meta" + (bio ? "" : " no-about")}>
                <div>
                  <div className="profile-label">{t("memberSince")}</div>
                  <div className="profile-value">{memberSince ?? "—"}</div>
                </div>
                <div>
                  <div className="profile-label">{t("localTime")}</div>
                  <div className="profile-value">{localTime ?? "—"}</div>
                </div>
              </div>
              {connections.length > 0 && (
                <div className="profile-mutual">
                  <div className="profile-label">{t("connections")}</div>
                  <div className="profile-chips">
                    {connections.map((c) => {
                      const inner = (
                        <>
                          <Icon name={c.provider === "steam" ? "steam" : "github"} size={14} />
                          <span>{c.displayName || c.provider}</span>
                        </>
                      );
                      return c.url ? (
                        <a
                          key={c.provider}
                          className="profile-chip profile-conn"
                          href={c.url}
                          target="_blank"
                          rel="noreferrer"
                        >
                          {inner}
                        </a>
                      ) : (
                        <span key={c.provider} className="profile-chip profile-conn">
                          {inner}
                        </span>
                      );
                    })}
                  </div>
                </div>
              )}
              {profile && profile.mutualServers.length > 0 && (
                <div className="profile-mutual">
                  <div className="profile-label">{t("mutualServers", { count: profile.mutualServers.length })}</div>
                  <div className="profile-chips">
                    {profile.mutualServers.map((s) => (
                      <span key={s.id} className="profile-chip" title={s.name}>
                        <ServerLogo serverId={s.id} name={s.name} accent={s.accentColor} className="profile-chip-logo" />
                        <span>{s.name}</span>
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
};

export default UserProfileModal;
