import React from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { UserAvatar } from "../../../shared/ui/Avatar";
import { useAuthStore } from "../../auth/store/authStore";
import { useFriendshipStore } from "../store/friendshipStore";
import { useBlockStore } from "../store/blockStore";
import UserEmojiStatus from "./UserEmojiStatus";
import { useAvatarUrl } from "../../../shared/lib/avatarCache";
import { openReport } from "../../moderation/store/reportModalStore";
import { usePresenceStore } from "../store/presenceStore";
import { useProfileModalStore } from "../store/profileModalStore";
import { useUserProfileDetails } from "../hooks/useUserProfileDetails";
import ProfileFacts from "./ProfileFacts";
import { useDialogFocus } from "../../../shared/hooks/useDialogFocus";
import { useUserNamesStore } from "../store/userNamesStore";
import { colorForId } from "../../../shared/lib/avatar";
import i18n from "../../../i18n";
import { shareProfile } from "../lib/profileLink";
import { displayUserName } from "../../../shared/lib/displayName";

interface Props {
  onOpenDm: (userId: string, name?: string) => void;
  onCall: (userId: string, video?: boolean) => void;
  onRemoveFriend: (userId: string, name: string) => void;
  onBlockUser: (userId: string, name: string) => void;
  onEditProfile: () => void;
}

const UserProfileModal: React.FC<Props> = ({ onOpenDm, onCall, onRemoveFriend, onBlockUser, onEditProfile }) => {
  const { t } = useTranslation("profile");
  const userId = useProfileModalStore((s) => s.userId);
  const close = useProfileModalStore((s) => s.close);
  const dialogRef = React.useRef<HTMLDivElement>(null);
  useDialogFocus(dialogRef, !!userId, userId);
  const selfId = useAuthStore((s) => s.user?.id);
  const selfName = useAuthStore((s) => s.user?.name);
  const { profile, failed, blockedByMe, blockedMe, friendshipStatus, bio } = useUserProfileDetails(userId);
  const onlineUsers = usePresenceStore((s) => s.online);
  const selfOnline = usePresenceStore((s) => s.selfOnline);
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const [menuOpen, setMenuOpen] = React.useState(false);
  const bannerUrl = useAvatarUrl("banner", blockedMe ? null : userId);
  const avatarUrl = useAvatarUrl("user", userId);
  const [avatarOpen, setAvatarOpen] = React.useState(false);
  const avatarOpenRef = React.useRef(avatarOpen);
  avatarOpenRef.current = avatarOpen;
  const [copied, setCopied] = React.useState(false);
  const [linkCopied, setLinkCopied] = React.useState(false);
  const menuRef = React.useRef<HTMLDivElement>(null);


  React.useEffect(() => {
    setMenuOpen(false);
    setCopied(false);
    setLinkCopied(false);
    setAvatarOpen(false);
  }, [userId]);


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
  const name = nameOverrides[userId] ?? profile?.name ?? (isSelf ? (selfName ?? "") : "");
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

  const shareLink = async () => {
    setMenuOpen(false);
    try {
      const outcome = await shareProfile(userId, name || undefined);
      if (outcome !== "copied") return;
      setLinkCopied(true);
      setTimeout(() => setLinkCopied(false), 1500);
    } catch {
      setLinkCopied(false);
    }
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
              : { background: `linear-gradient(180deg, color-mix(in srgb, ${colorForId(userId)} 70%, var(--surface-dialog)) 0, color-mix(in srgb, ${colorForId(userId)} 42%, var(--surface-dialog)) 40%, color-mix(in srgb, ${colorForId(userId)} 16%, var(--surface-dialog)) 75%, var(--surface-dialog) 100%)` }
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
                <div className="profile-name">{name ? displayUserName(name) : inactive ? t("deletedUser") : "…"}</div>
                {!inactive && !blockedMe && <UserEmojiStatus userId={userId} size="lg" />}
              </div>
              <div className="profile-handle">
                {name && !inactive ? `@${name} · ` : ""}
                {inactive ? t("inactive") : online ? t("online") : t("offline")}
              </div>
            </>
          )}

          {isSelf && !failed && (
            <div className="profile-actions">
              <button type="button" className="profile-btn primary" onClick={() => { close(); onEditProfile(); }}>
                <Icon name="edit" size={15} />
                <span className="profile-btn-label">{t("editProfile")}</span>
              </button>
              <button type="button" className="profile-btn" onClick={() => void shareLink()}>
                <Icon name="link" size={15} />
                <span className="profile-btn-label">{linkCopied ? t("linkCopied") : t("shareProfile")}</span>
              </button>
            </div>
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
                    <button type="button" onClick={() => void shareLink()}>{t("shareProfile")}</button>
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

          {!isSelf && linkCopied && (
            <div className="profile-link-note" role="status">
              <Icon name="check" size={13} />
              <span>{t("linkCopied")}</span>
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
              <ProfileFacts profile={profile} bio={bio} blockedMe={blockedMe} />
            </>
          )}
        </div>
      </div>
    </div>
  );
};

export default UserProfileModal;
