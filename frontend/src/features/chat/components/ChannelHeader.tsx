import React from "react";
import { useActivityStore } from "../../social/store/activityStore";
import { useCallStore } from "../../voice/store/callStore";

import i18n from "../../../i18n";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import { UserAvatar } from "../../../shared/ui/Avatar";
import Icon from "../../../shared/ui/Icon";
import { openReport } from "../../moderation/store/reportModalStore";
import type { ChannelMeta, DmMeta } from "../types";
import { useDetailsPanelStore } from "../store/detailsPanelStore";
import UserBadge from "../../subscription/components/UserBadge";
import { useHiddenDmStore } from "../../dm/store/hiddenDmStore";
import { useClearDmDialog } from "../../dm/components/ClearDmDialog";

export interface ChannelHeaderProps {
  channel?: ChannelMeta;
  dm?: DmMeta;
  blocked?: boolean;
  blockedBy?: boolean;
  onUnblockUser?: () => void;
  searchActive?: boolean;
  onToggleSearch?: () => void;
  onRemoveFriend?: () => void;
  onBlockUser?: () => void;
}

export const ChannelHeader: React.FC<ChannelHeaderProps> = ({
  channel,
  dm,
  blocked,
  blockedBy,
  onUnblockUser,
  searchActive,
  onToggleSearch,
  onRemoveFriend,
  onBlockUser,
}) => {
  const startDmCall = (video: boolean) => {
    if (!dm?.friendId) return;
    useCallStore.getState().callFriend(dm.friendId, video);
  };
  const dmActivity = useActivityStore((s) => (dm ? s.byUser[dm.friendId] : undefined));
  const [moreOpen, setMoreOpen] = React.useState(false);
  const dmHidden = useHiddenDmStore((s) => (dm ? s.hidden.includes(dm.friendId) : false));
  const detailsOpen = useDetailsPanelStore((s) => s.open);
  const detailsToggle = (
    <button
      type="button"
      className={"icon-btn" + (detailsOpen ? " on" : "")}
      title={i18n.t(detailsOpen ? "details.close" : "details.open", { ns: "chat" })}
      aria-label={i18n.t(detailsOpen ? "details.close" : "details.open", { ns: "chat" })}
      aria-pressed={detailsOpen}
      onClick={() => useDetailsPanelStore.getState().toggle()}
    >
      <Icon name="panel-right" size={16} />
    </button>
  );
  const moreRef = React.useRef<HTMLDivElement>(null);

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
          <UserBadge userId={dm.friendId} size="sm" />
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
          <div className="header-call-group">
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
          </div>
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
          {detailsToggle}
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
                <button
                  type="button"
                  className="pop-item"
                  role="menuitem"
                  onClick={() => {
                    setMoreOpen(false);
                    void useHiddenDmStore
                      .getState()
                      .setHidden(dm.friendId, !dmHidden)
                      .catch((e) => console.warn("[ChannelHeader] hide failed:", e));
                  }}
                >
                  <Icon name={dmHidden ? "eye" : "eye-off"} size={15} />
                  <span className="body">{i18n.t(dmHidden ? "unhideChat" : "hideChat", { ns: "sidebar" })}</span>
                </button>
                <button
                  type="button"
                  className="pop-item danger"
                  role="menuitem"
                  onClick={() => {
                    setMoreOpen(false);
                    useClearDmDialog.getState().open({ id: dm.friendId, name: dm.name });
                  }}
                >
                  <Icon name="trash" size={15} />
                  <span className="body">{i18n.t("clearChat", { ns: "sidebar" })}</span>
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
        {detailsToggle}
      </div>
    </header>
  );
};

export default ChannelHeader;
