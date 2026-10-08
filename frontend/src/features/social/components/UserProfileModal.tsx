import React from "react";
import { useQuery } from "@tanstack/react-query";
import { queryClient, queryKeys } from "../../../shared/query/queryClient";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { CommunityLogo, UserAvatar } from "../../../shared/ui/Avatar";
import { homeAPI, type PublicProfile } from "../../home/api/home";
import { useAuthStore } from "../../auth/store/authStore";
import { useFriendshipStore } from "../store/friendshipStore";
import { blockGeneration, useBlockStore } from "../store/blockStore";
import ProBadge from "../../subscription/components/ProBadge";
import { useBadge } from "../../subscription/hooks/useBadge";
import { useAvatarUrl } from "../../../shared/lib/avatarCache";
import { openReport } from "../../moderation/store/reportModalStore";
import { usePresenceStore } from "../store/presenceStore";
import { loadActivitiesFor, useActivityStore } from "../store/activityStore";
import { useProfileModalStore } from "../store/profileModalStore";
import { useDialogFocus } from "../../../shared/hooks/useDialogFocus";
import { userAPI, type Connection } from "../api/user";
import { useUserNamesStore } from "../store/userNamesStore";
import { profileVersion, useUserProfileStore } from "../store/userProfileStore";
import { colorForId } from "../../../shared/lib/avatar";
import i18n, { currentLocale } from "../../../i18n";

interface Props {
  onOpenDm: (userId: string, name?: string) => void;
  onCall: (userId: string, video?: boolean) => void;
  onRemoveFriend: (userId: string, name: string) => void;
  onBlockUser: (userId: string, name: string) => void;
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

const UserProfileModal: React.FC<Props> = ({ onOpenDm, onCall, onRemoveFriend, onBlockUser }) => {
  const { t } = useTranslation("profile");
  const userId = useProfileModalStore((s) => s.userId);
  const close = useProfileModalStore((s) => s.close);
  const dialogRef = React.useRef<HTMLDivElement>(null);
  useDialogFocus(dialogRef, !!userId, userId);
  const selfId = useAuthStore((s) => s.user?.id);
  const friendStatuses = useFriendshipStore((s) => s.statusById);
  const blockedByMe = useBlockStore((s) => (userId ? !!s.blocked[userId] : false));
  const blockedMe = useBlockStore((s) => (userId ? !!s.blockedBy[userId] : false));
  const onlineUsers = usePresenceStore((s) => s.online);
  const selfOnline = usePresenceStore((s) => s.selfOnline);
  const gameActivity = useActivityStore((s) => (userId ? s.byUser[userId] : undefined));
  React.useEffect(() => {
    if (userId) void loadActivitiesFor([userId]);
  }, [userId]);
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const cachedBio = useUserProfileStore((s) => (userId ? s.bios[userId]?.bio : undefined));
  const [menuOpen, setMenuOpen] = React.useState(false);
  const badge = useBadge(userId);
  const bannerUrl = useAvatarUrl("banner", badge && !blockedMe ? userId : null);
  const avatarUrl = useAvatarUrl("user", userId);
  const [avatarOpen, setAvatarOpen] = React.useState(false);
  const avatarOpenRef = React.useRef(avatarOpen);
  avatarOpenRef.current = avatarOpen;
  const [copied, setCopied] = React.useState(false);
  const menuRef = React.useRef<HTMLDivElement>(null);
  useMinuteTick();

  const friendshipStatus = userId ? friendStatuses[userId] : undefined;

  React.useEffect(() => {
    setMenuOpen(false);
    setCopied(false);
    setAvatarOpen(false);
  }, [userId]);

  const connectionsQuery = useQuery(
    {
      queryKey: queryKeys.userConnections(userId ?? ""),
      queryFn: () => userAPI.userConnections(userId as string),
      enabled: !!userId && !blockedMe,
    },
    queryClient,
  );
  const connections: Connection[] = userId && !blockedMe ? (connectionsQuery.data ?? []) : [];

  const profileQuery = useQuery(
    {
      queryKey: [...queryKeys.publicProfile(userId ?? ""), friendshipStatus ?? null, blockedByMe, blockedMe],
      queryFn: async () => {
        const id = userId as string;
        const gen = blockGeneration(id);
        const session = useAuthStore.getState().user?.id;
        const p = await homeAPI.profile(id);
        if (!session || useAuthStore.getState().user?.id !== session) throw new Error("SESSION_CHANGED");
        useUserProfileStore.getState().apply(id, p.bio, profileVersion(p.profileUpdatedAt));
        useBlockStore.getState().applyRelation(id, { blocked: p.blocked, blockedBy: p.blockedBy }, gen);
        return p;
      },
      enabled: !!userId,
      staleTime: 0,
    },
    queryClient,
  );
  const profile: PublicProfile | null = userId ? (profileQuery.data ?? null) : null;
  const failed = profileQuery.isError && !profile;

  React.useEffect(() => {
    if (!userId) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopImmediatePropagation();
        if (avatarOpenRef.current) setAvatarOpen(false);
        else close();
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

  const isSelf = userId === selfId;
  const isFriend = friendshipStatus === "accepted";
  const online = isSelf ? selfOnline : !blockedByMe && !blockedMe && !!onlineUsers[userId];
  const name = nameOverrides[userId] ?? profile?.name ?? "";
  const bio = blockedMe ? "" : (cachedBio ?? profile?.bio ?? "");
  const locale = currentLocale();
  const localTime = profile && !blockedMe ? formatLocalTime(profile.timezone, locale) : null;
  const memberSince = profile && !blockedMe ? formatMemberSince(profile.memberSince, locale) : null;
  const inactive = !!profile && !profile.active;
  const blockedEitherWay = blockedByMe || blockedMe;
  const canMessage = !isSelf && !inactive && !blockedEitherWay && (profile ? profile.acceptsMessages : isFriend);
  const canCall = !isSelf && !inactive && !blockedEitherWay && (profile ? profile.acceptsCalls : isFriend);

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
      <div
        ref={dialogRef}
        className="profile-window"
        onMouseDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
      >
        <div
          className={"profile-band" + (bannerUrl ? " profile-banner-image" : "")}
          style={
            bannerUrl
              ? { backgroundImage: `url(${bannerUrl})` }
              : { background: `color-mix(in srgb, ${colorForId(userId)} 28%, var(--surface-raised))` }
          }
        >
          <button type="button" className="profile-close" onClick={close} aria-label={t("close")}>
            <Icon name="x" size={15} />
          </button>
        </div>
        {avatarOpen &&
          avatarUrl &&
          createPortal(
            <div
              className="image-viewer-overlay"
              onClick={(e) => {
                e.stopPropagation();
                setAvatarOpen(false);
              }}
            >
              <img src={avatarUrl} alt={name} className="profile-avatar-full" />
            </div>,
            document.body,
          )}
        <div className="profile-body">
          <div className="profile-avatar-row">
            <span className="profile-avatar-wrap">
              {avatarUrl ? (
                <button
                  type="button"
                  className="profile-avatar-open"
                  title={t("openAvatar")}
                  aria-label={t("openAvatar")}
                  onClick={() => setAvatarOpen(true)}
                >
                  <UserAvatar userId={userId} name={name || "?"} className="profile-avatar" style={{ background: colorForId(userId) }} />
                </button>
              ) : (
                <UserAvatar userId={userId} name={name || "?"} className="profile-avatar" style={{ background: colorForId(userId) }} />
              )}
              <span className={"profile-presence " + (online ? "online" : "offline")} />
            </span>
            {profile?.voiceRoom && !blockedMe && (
              <span className="profile-voice-pill" title={profile.voiceRoom.communityName}>
                <Icon name="volume" size={13} />
                {t("inVoice", { room: profile.voiceRoom.channelName })}
              </span>
            )}
          </div>

          {failed ? (
            <div className="profile-name">{t("unavailable")}</div>
          ) : (
            <>
              <div className="profile-name-row">
                <div className="profile-name">{name || (inactive ? t("deletedUser") : "…")}</div>
                {badge && !inactive && !blockedMe && (
                  <ProBadge tier={badge.tier} badgeStyle={badge.badgeStyle} memberNumber={badge.memberNumber} size="md" />
                )}
              </div>
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
              {!isFriend && !blockedEitherWay && (
                friendshipStatus === "pending_outgoing" ? (
                  <button type="button" className="profile-btn icon" title={t("requestSent")} disabled>
                    <Icon name="check" size={22} />
                  </button>
                ) : friendshipStatus === "pending_incoming" ? (
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
                <span className="profile-btn-label">{t("message")}</span>
              </button>
              <button type="button" className="profile-btn" disabled={!canCall} onClick={() => { close(); onCall(userId); }}>
                <Icon name="phone" size={15} />
                <span className="profile-btn-label">{t("call")}</span>
              </button>
              <button type="button" className="profile-btn" disabled={!canCall} onClick={() => { close(); onCall(userId, true); }}>
                <Icon name="video" size={15} />
                <span className="profile-btn-label">{t("videoCall")}</span>
              </button>
              <div className="profile-more" ref={menuRef}>
                <button type="button" className="profile-btn icon" aria-label={t("more")} onClick={() => setMenuOpen((v) => !v)}>
                  <Icon name="more" size={16} />
                </button>
                {menuOpen && (
                  <div className="profile-menu">
                    <button type="button" onClick={() => void copyLogin()} disabled={!name}>{copied ? t("copied") : t("copyLogin")}</button>
                    <button
                      type="button"
                      onClick={() => {
                        setMenuOpen(false);
                        close();
                        openReport({ type: "user", id: userId, name });
                      }}
                    >
                      {i18n.t("action", { ns: "report" })}
                    </button>
                    {isFriend && (
                      <button type="button" className="danger" onClick={() => { setMenuOpen(false); close(); onRemoveFriend(userId, name); }}>
                        {t("removeFriend")}
                      </button>
                    )}
                    {blockedByMe ? (
                      <button type="button" onClick={() => { setMenuOpen(false); void useBlockStore.getState().unblock(userId).catch(() => {}); }}>
                        {t("unblock")}
                      </button>
                    ) : (
                      <button type="button" className="danger" onClick={() => { setMenuOpen(false); close(); onBlockUser(userId, name); }}>
                        {t("block")}
                      </button>
                    )}
                  </div>
                )}
              </div>
            </div>
          )}

          {!isSelf && !failed && blockedEitherWay && (
            <div className="profile-block-note">
              <Icon name="info" size={14} />
              <span>{blockedByMe ? t("blockedNotice") : t("blockedByNotice")}</span>
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
                {localTime && (
                  <div>
                    <div className="profile-label">{t("localTime")}</div>
                    <div className="profile-value">{localTime}</div>
                  </div>
                )}
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
              {profile && !blockedMe && profile.mutualCommunities.length > 0 && (
                <div className="profile-mutual">
                  <div className="profile-label">{t("mutualCommunities", { count: profile.mutualCommunities.length })}</div>
                  <div className="profile-chips">
                    {profile.mutualCommunities.map((s) => (
                      <span key={s.id} className="profile-chip" title={s.name}>
                        <CommunityLogo communityId={s.id} name={s.name} accent={s.accentColor} className="profile-chip-logo" />
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
