import React from "react";
import { useNotificationSettingsStore } from "../../settings/store/notificationSettingsStore";
import { useActivityStore } from "../../social/store/activityStore";
import { ApiError } from "../../../shared/lib/apiError";
import { useCallStore } from "../../voice/store/callStore";

import i18n from "../../../i18n";
import { useNotificationPrefsStore } from "../../settings/store/notificationPrefsStore";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import { UserAvatar } from "../../../shared/ui/Avatar";
import Icon from "../../../shared/ui/Icon";
import { openReport } from "../../moderation/store/reportModalStore";
import type { ChannelMeta, DmMeta } from "../types";

export interface ChannelHeaderProps {
  channel?: ChannelMeta;
  dm?: DmMeta;
  blocked?: boolean;
  blockedBy?: boolean;
  onUnblockUser?: () => void;
  membersOpen?: boolean;
  searchActive?: boolean;
  onToggleSearch?: () => void;
  onToggleMembers?: () => void;
  sharedOpen?: boolean;
  onToggleShared?: () => void;
  onRemoveFriend?: () => void;
  onBlockUser?: () => void;
}

export const ChannelHeader: React.FC<ChannelHeaderProps> = ({
  channel,
  dm,
  blocked,
  blockedBy,
  onUnblockUser,
  membersOpen,
  searchActive,
  onToggleSearch,
  onToggleMembers,
  sharedOpen,
  onToggleShared,
  onRemoveFriend,
  onBlockUser,
}) => {
  const startDmCall = (video: boolean) => {
    if (!dm?.friendId) return;
    const cs = useCallStore.getState();
    const inCallWithFriend =
      !!cs.activeCallId &&
      (cs.peerId === dm.friendId ||
        dm.friendId in cs.participants ||
        cs.ringing.includes(dm.friendId));
    if (inCallWithFriend) {
      cs.setFocused(true);
      return;
    }
    cs.start([dm.friendId], { video }).catch((e) => {
      const apiErr = e instanceof ApiError ? e : null;
      cs.setCallError(
        apiErr?.code === "CALL_NOT_ALLOWED"
          ? i18n.t("callNotAllowed", { ns: "call" })
          : i18n.t("callFailed", { ns: "call" }),
      );
    });
  };
  const [bellOpen, setBellOpen] = React.useState(false);
  const dmActivity = useActivityStore((s) => (dm ? s.byUser[dm.friendId] : undefined));
  const globalMentions = useNotificationSettingsStore((s) => s.mentions);
  const notifLevel = useNotificationPrefsStore((s) =>
    dm
      ? (s.dmLevels[dm.friendId] ?? (globalMentions === "none" ? "none" : "all"))
      : channel
        ? (s.channelLevels[channel.id] ?? globalMentions)
        : "all",
  );
  const bellRef = React.useRef<HTMLDivElement>(null);
  const bellButtonRef = React.useRef<HTMLButtonElement>(null);
  const [moreOpen, setMoreOpen] = React.useState(false);
  const moreRef = React.useRef<HTMLDivElement>(null);

  const applyNotifLevel = (level: "all" | "mentions" | "none") => {
    const prefs = useNotificationPrefsStore.getState();
    const action = dm
      ? level !== "mentions" && prefs.setDmLevel(dm.friendId, level)
      : channel
        ? prefs.setChannelLevel(channel.id, level)
        : null;
    if (action) {
      action.catch((e) =>
        console.warn("[notifications] failed to save preference:", e),
      );
    }
    setBellOpen(false);
  };

  const bellPopover = (
    title: string,
    options: {
      k: "all" | "mentions" | "none";
      ttl: string;
      sub: string;
    }[],
  ) => (
    <div className="popover-wrap" ref={bellRef}>
      <button
        ref={bellButtonRef}
        type="button"
        className={"icon-btn" + (bellOpen ? " on" : "")}
        title={i18n.t("notifications.bell", { ns: "chat" })}
        onClick={() => setBellOpen((o) => !o)}
      >
        <Icon name="bell" size={15} />
        {notifLevel !== "all" && (
          <span
            className="nb-dot"
            style={{ background: notifLevel === "none" ? "#ff453a" : "#ff9f0a" }}
          />
        )}
      </button>
      {bellOpen && (
        <div className="popover" onMouseDown={(e) => e.stopPropagation()}>
          <div className="pop-head">{title}</div>
          {options.map((opt) => (
            <button
              type="button"
              key={opt.k}
              className="pop-item"
              onClick={() => applyNotifLevel(opt.k)}
            >
              <span className="check">
                {notifLevel === opt.k && <Icon name="check" size={14} />}
              </span>
              <span className="body">
                <div className="ttl">{opt.ttl}</div>
                <div className="sub">{opt.sub}</div>
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );

  React.useEffect(() => {
    if (!moreOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (moreRef.current && !moreRef.current.contains(e.target as Node)) {
        setMoreOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMoreOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [moreOpen]);

  React.useEffect(() => {
    if (!bellOpen) return;
    const onDoc = (e: MouseEvent) => {
      if (bellRef.current && !bellRef.current.contains(e.target as Node)) {
        setBellOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [bellOpen]);

  React.useEffect(() => {
    if (!bellOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setBellOpen(false);
        bellButtonRef.current?.focus();
        return;
      }
      if (e.key !== "Tab" || !bellRef.current) return;
      const focusables = bellRef.current.querySelectorAll<HTMLElement>(
        'button:not([tabindex="-1"])',
      );
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement as HTMLElement | null;
      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [bellOpen]);

  if (dm) {
    return (
      <header className="main-header">
        <button
          type="button"
          className="dm-peer-btn"
          title={i18n.t("viewProfile", { ns: "profile" })}
          onClick={() => useProfileModalStore.getState().open(dm.friendId)}
        >
          <UserAvatar
            userId={dm.friendId}
            name={dm.name}
            style={{
              width: 20,
              height: 20,
              borderRadius: "50%",
              background: dm.color,
              color: "#fff",
              fontSize: 10,
              fontWeight: 700,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          />
          <span className="ch-name">{dm.name}</span>
          <Icon name="chev" size={10} className="dm-peer-chev" />
        </button>
        <span className="sep" />
        <span className="topic">
          {dm.online && dmActivity
            ? i18n.t("playing", { ns: "chat", game: dmActivity.name })
            : dm.online
              ? i18n.t("activeNow", { ns: "chat" })
              : i18n.t("offline", { ns: "chat" })}
        </span>
        <div className="right">
          <button
            type="button"
            className="icon-btn"
            title={i18n.t("voiceCallInvite", { ns: "chat" })}
            disabled={blocked || blockedBy}
            onClick={() => startDmCall(false)}
          >
            <Icon name="phone" size={16} />
          </button>
          <button
            type="button"
            className="icon-btn"
            title={i18n.t("videoCallInvite", { ns: "chat" })}
            disabled={blocked || blockedBy}
            onClick={() => startDmCall(true)}
          >
            <Icon name="video" size={16} />
          </button>
          {dm.isFriend !== false && bellPopover(
            i18n.t("notifications.dmTitle", { ns: "chat", name: dm.name }),
            [
              { k: "all", ttl: i18n.t("notifications.all", { ns: "chat" }), sub: i18n.t("notifications.allHint", { ns: "chat" }) },
              { k: "none", ttl: i18n.t("notifications.nothing", { ns: "chat" }), sub: i18n.t("notifications.dmNothingHint", { ns: "chat" }) },
            ],
          )}
          {onToggleShared && (
            <button
              type="button"
              className={"icon-btn" + (sharedOpen ? " on" : "")}
              title={i18n.t(sharedOpen ? "shared.close" : "shared.title", { ns: "chat" })}
              aria-pressed={!!sharedOpen}
              onClick={onToggleShared}
            >
              <Icon name="paperclip" size={15} />
            </button>
          )}
          {onToggleSearch && (
            <button
              type="button"
              className={"icon-btn" + (searchActive ? " on" : "")}
              title={searchActive ? i18n.t("closeSearch", { ns: "chat" }) : i18n.t("searchWithUser", { ns: "chat", user: dm.name })}
              onClick={onToggleSearch}
            >
              <Icon name="search" size={15} />
            </button>
          )}
          <div className="popover-wrap" ref={moreRef}>
            <button
              type="button"
              className={"icon-btn" + (moreOpen ? " on" : "")}
              title={i18n.t("more", { ns: "chat" })}
              aria-haspopup="menu"
              aria-expanded={moreOpen}
              onClick={() => setMoreOpen((o) => !o)}
            >
              <Icon name="more" size={16} />
            </button>
            {moreOpen && (
              <div className="popover ch-more-menu" role="menu">
                <button
                  type="button"
                  className="pop-item"
                  role="menuitem"
                  onClick={() => {
                    setMoreOpen(false);
                    openReport({ type: "user", id: dm.friendId, name: dm.name });
                  }}
                >
                  <Icon name="flag" size={15} />
                  <span className="body">{i18n.t("action", { ns: "report" })}</span>
                </button>
                {onRemoveFriend && dm.isFriend !== false && !blocked && (
                  <button
                    type="button"
                    className="pop-item danger"
                    role="menuitem"
                    onClick={() => {
                      setMoreOpen(false);
                      onRemoveFriend();
                    }}
                  >
                    <Icon name="user-check" size={15} />
                    <span className="body">{i18n.t("removeFriend", { ns: "chat" })}</span>
                  </button>
                )}
                {(blocked ? onUnblockUser : onBlockUser) && (
                  <button
                    type="button"
                    className={"pop-item" + (blocked ? "" : " danger")}
                    role="menuitem"
                    onClick={() => {
                      setMoreOpen(false);
                      (blocked ? onUnblockUser : onBlockUser)?.();
                    }}
                  >
                    <Icon name="shield" size={15} />
                    <span className="body">
                      {i18n.t(blocked ? "unblockUser" : "blockUser", { ns: "chat" })}
                    </span>
                  </button>
                )}
              </div>
            )}
          </div>
        </div>
      </header>
    );
  }

  if (!channel) return null;

  return (
    <header className="main-header">
      <span className="ch-ico">
        <Icon name="hash" size={16} />
      </span>
      <span className="ch-name">{channel.name}</span>
      {channel.topic && (
        <>
          <span className="sep" />
          <span className="topic">{channel.topic}</span>
        </>
      )}
      <div className="right">
        {bellPopover(
          i18n.t("notifications.title", { ns: "chat", channel: channel.name }),
          [
            { k: "all", ttl: i18n.t("notifications.all", { ns: "chat" }), sub: i18n.t("notifications.allHint", { ns: "chat" }) },
            { k: "mentions", ttl: i18n.t("notifications.mentions", { ns: "chat" }), sub: i18n.t("notifications.mentionsHint", { ns: "chat" }) },
            { k: "none", ttl: i18n.t("notifications.nothing", { ns: "chat" }), sub: i18n.t("notifications.nothingHint", { ns: "chat" }) },
          ],
        )}
        {onToggleMembers && (
          <button
            type="button"
            className={"icon-btn" + (membersOpen ? " on" : "")}
            title={membersOpen ? "Hide members" : "Show members"}
            onClick={onToggleMembers}
          >
            <Icon name="users" size={15} />
          </button>
        )}
        {onToggleShared && (
          <button
            type="button"
            className={"icon-btn" + (sharedOpen ? " on" : "")}
            title={i18n.t(sharedOpen ? "shared.close" : "shared.title", { ns: "chat" })}
            aria-pressed={!!sharedOpen}
            onClick={onToggleShared}
          >
            <Icon name="paperclip" size={15} />
          </button>
        )}
        {onToggleSearch && (
          <button
            type="button"
            className={"icon-btn" + (searchActive ? " on" : "")}
            title={searchActive ? i18n.t("closeSearch", { ns: "chat" }) : i18n.t("search", { ns: "common" })}
            onClick={onToggleSearch}
          >
            <Icon name="search" size={15} />
          </button>
        )}
      </div>
    </header>
  );
};

export default ChannelHeader;
