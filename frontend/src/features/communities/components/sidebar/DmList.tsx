import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import type { DirectConversation } from "../../../dm/types";
import { useActivityStore } from "../../../social/store/activityStore";
import UserBadge from "../../../subscription/components/UserBadge";
import { formatListTime, msUntilNextDay } from "../../../dm/lib/directOrder";
import { useProfileModalStore } from "../../../social/store/profileModalStore";
import { isBlockedEitherWay } from "../../../social/store/blockStore";
import { useFriendshipStore } from "../../../social/store/friendshipStore";
import { useDmPreviewStore } from "../../../dm/store/dmPreviewStore";
import { dmLevelOf, useNotificationPrefsStore } from "../../../settings/store/notificationPrefsStore";
import { MAX_PINNED_CONVERSATIONS, useConversationPinsStore } from "../../store/conversationPinsStore";
import { useNotificationSettingsStore } from "../../../settings/store/notificationSettingsStore";
import {
  openContextMenu,
  openContextMenuAt,
} from "../../../../shared/ui/contextMenu/contextMenuStore";
import type { MenuEntry } from "../../../../shared/ui/contextMenu/contextMenuStore";
import { useLongPress } from "../../../../shared/ui/contextMenu/useLongPress";
import { useCallStore } from "../../../voice/store/callStore";
import { useHiddenDmStore } from "../../../dm/store/hiddenDmStore";
import { useClearDmDialog } from "../../../dm/components/ClearDmDialog";

interface DmListProps {
  directs: DirectConversation[];
  selectedDmId: string | null;
  unreadByDm: Record<string, number>;
  pendingCount: number;
  hasMoreDirects?: boolean;
  onLoadMoreDirects?: () => void;
  onAddFriend: () => void;
  onOpenPending: () => void;
  onPick: (dm: DirectConversation) => void;
  onCall?: (dm: DirectConversation) => void;
  compact?: boolean;
}

const DmList: React.FC<DmListProps> = ({
  directs,
  selectedDmId,
  unreadByDm,
  pendingCount,
  hasMoreDirects,
  onLoadMoreDirects,
  onAddFriend,
  onOpenPending,
  onPick,
  onCall,
  compact,
}) => {
  const { t, i18n } = useTranslation(["sidebar", "common"]);
  const activities = useActivityStore((s) => s.byUser);
  const dmLevels = useNotificationPrefsStore((s) => s.dmLevels);
  const pinnedDms = useConversationPinsStore((s) => s.dms);
  const pinsLoaded = useConversationPinsStore((s) => s.loaded);
  const friends = useFriendshipStore((s) => s.friends);
  const friendsComplete = useFriendshipStore((s) => s.loaded && !s.nextCursor);
  const previews = useDmPreviewStore((s) => s.byFriend);
  const previewsComplete = useDmPreviewStore((s) => s.loaded && !s.nextCursor);
  const pinStillReachable = (id: string) =>
    !isBlockedEitherWay(id) &&
    (!!previews[id] ||
      friends.some((f) => f.friendId === id && f.status === "accepted") ||
      !(friendsComplete && previewsComplete));
  const accessiblePins = () => pinnedDms.filter(pinStillReachable);
  const globalMentions = useNotificationSettingsStore((s) => s.mentions);
  const joinableByPeer = useCallStore((s) => s.joinableByPeer);
  const callPeer = useCallStore((s) => (s.activeCallId ? s.peerId : null));
  const inCallWith = (id: string) => !!joinableByPeer[id] || callPeer === id;
  const hiddenIds = useHiddenDmStore((s) => s.hidden);
  const [showHidden, setShowHidden] = React.useState(false);
  const hasHidden = hiddenIds.length > 0;
  React.useEffect(() => {
    if (!hasHidden) setShowHidden(false);
  }, [hasHidden]);
  const hiddenSet = new Set(hiddenIds);
  const shown = directs.filter((d) => hiddenSet.has(d.id) === showHidden);
  const hiddenUnread = hiddenIds.reduce((sum, id) => sum + (unreadByDm[id] ?? 0), 0);
  const [now, setNow] = React.useState(() => new Date());
  React.useEffect(() => {
    const timer = setTimeout(() => setNow(new Date()), msUntilNextDay(now));
    return () => clearTimeout(timer);
  }, [now]);
  const dmMenu = (d: DirectConversation): MenuEntry[] => [
    { kind: "item", id: "message", label: t("memberMessage"), icon: "message", onSelect: () => onPick(d) },
    ...(onCall && !isBlockedEitherWay(d.id)
      ? [{ kind: "item" as const, id: "call", label: t("callFriend"), icon: "phone" as const, onSelect: () => onCall(d) }]
      : []),
    {
      kind: "item",
      id: "pin",
      label: t(pinnedDms.includes(d.id) ? "unpinChat" : "pinChat"),
      icon: "pin",
      disabled: !pinsLoaded || (!pinnedDms.includes(d.id) && accessiblePins().length >= MAX_PINNED_CONVERSATIONS),
      onSelect: () => {
        void useConversationPinsStore
          .getState()
          .setPinned("dms", d.id, !pinnedDms.includes(d.id), pinStillReachable)
          .catch((e) => console.warn("[DmList] pin failed:", e));
      },
    },
    ...(d.isFriend === false
      ? []
      : [
          {
            kind: "item" as const,
            id: "notifications",
            label: t(dmLevelOf(dmLevels, d.id, globalMentions) === "none" ? "unmuteChat" : "muteChat"),
            icon: dmLevelOf(dmLevels, d.id, globalMentions) === "none" ? ("bell" as const) : ("bell-off" as const),
            onSelect: () => {
              const next = dmLevelOf(dmLevels, d.id, globalMentions) === "none" ? "all" : "none";
              void useNotificationPrefsStore
                .getState()
                .setDmLevel(d.id, next)
                .catch((e) => console.warn("[DmList] notification level failed:", e));
            },
          },
        ]),
    {
      kind: "item",
      id: "hide",
      label: t(hiddenSet.has(d.id) ? "unhideChat" : "hideChat"),
      icon: hiddenSet.has(d.id) ? "eye" : "eye-off",
      onSelect: () => {
        void useHiddenDmStore
          .getState()
          .setHidden(d.id, !hiddenSet.has(d.id))
          .catch((e) => console.warn("[DmList] hide failed:", e));
      },
    },
    { kind: "separator", id: "s1" },
    {
      kind: "item",
      id: "profile",
      label: t("memberProfile"),
      icon: "user",
      onSelect: () => useProfileModalStore.getState().open(d.id),
    },
    {
      kind: "item",
      id: "clear",
      label: t("clearChat"),
      icon: "trash",
      danger: true,
      onSelect: () => useClearDmDialog.getState().open({ id: d.id, name: d.name }),
    },
  ];
  const dmLongPress = useLongPress((point, el) => {
    const dm = directs.find((d) => d.id === el.dataset.dmId);
    return !!dm && openContextMenu(point, dmMenu(dm));
  });
  if (compact) {
    return (
      <div className="sb-mini-list" role="list" aria-label={t("directMessages")}>
        {(hasHidden || showHidden) && (
          <button
            type="button"
            className={"sb-mini sb-mini-more sb-mini-hidden" + (showHidden ? " active" : "")}
            title={showHidden ? t("hiddenChatsBack") : t("hiddenChats")}
            aria-label={showHidden ? t("hiddenChatsBack") : t("hiddenChats")}
            aria-pressed={showHidden}
            onClick={() => setShowHidden((v) => !v)}
          >
            <Icon name={showHidden ? "chev-left" : "eye-off"} size={16} />
            {!showHidden && hiddenUnread > 0 && (
              <span className="sb-mini-badge muted">{hiddenUnread > 99 ? "99+" : hiddenUnread}</span>
            )}
          </button>
        )}
        {shown.map((d) => {
          const unread = unreadByDm[d.id] ?? 0;
          const muted = dmLevelOf(dmLevels, d.id, globalMentions) === "none" || hiddenSet.has(d.id);
          return (
            <button
              key={d.id}
              type="button"
              role="listitem"
              className={"sb-mini" + (selectedDmId === d.id ? " active" : "") + (inCallWith(d.id) ? " in-call" : "")}
              data-dm-id={d.id}
              title={d.name}
              aria-label={unread > 0 ? t("unreadIn", { name: d.name, count: unread }) : d.name}
              aria-current={selectedDmId === d.id ? "true" : undefined}
              {...dmLongPress}
              onClick={() => onPick(d)}
              onContextMenu={(e) => openContextMenu(e, dmMenu(d))}
              onKeyDown={(e) => {
                if (e.key === "ContextMenu" || (e.key === "F10" && e.shiftKey)) {
                  e.preventDefault();
                  openContextMenuAt(e.currentTarget, dmMenu(d));
                }
              }}
            >
              <UserAvatar userId={d.id} name={d.name} className="sb-mini-ava" style={{ background: d.color }} />
              {d.online && <span className="sb-mini-online" />}
              {unread > 0 && (
                <span className={"sb-mini-badge" + (muted ? " muted" : "")}>{unread > 99 ? "99+" : unread}</span>
              )}
            </button>
          );
        })}
        {hasMoreDirects && onLoadMoreDirects && (
          <button type="button" className="sb-mini sb-mini-more" title={t("loadMore")} aria-label={t("loadMore")} onClick={onLoadMoreDirects}>
            <Icon name="more" size={16} />
          </button>
        )}
      </div>
    );
  }
  const renderRow = (d: DirectConversation) => {
    const unread = unreadByDm[d.id] ?? 0;
    const activity = d.online ? activities[d.id] : undefined;
    const muted = dmLevelOf(dmLevels, d.id, globalMentions) === "none";
    const quiet = muted || hiddenSet.has(d.id);
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
        data-dm-id={d.id}
        {...dmLongPress}
        onClick={() => onPick(d)}
        onContextMenu={(e) => openContextMenu(e, dmMenu(d))}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onPick(d);
          } else if (e.key === "ContextMenu" || (e.key === "F10" && e.shiftKey)) {
            e.preventDefault();
            openContextMenuAt(e.currentTarget, dmMenu(d));
          }
        }}
      >
        <div className={"sb-dm-ava-wrap" + (inCallWith(d.id) ? " in-call" : "")}>
          <UserAvatar
            userId={d.id}
            name={d.name}
            className="sb-dm-ava"
            style={{ background: d.color }}
          />
          {d.online && <span className="sb-dm-dot" />}
        </div>
        <div className="sb-dm-main">
          <div className="sb-dm-name">
            <span className="sb-dm-name-text">{d.name}</span>
            <UserBadge userId={d.id} size="sm" />
            {muted && (
              <span className="sb-dm-muted" title={t("dmMuted")} aria-label={t("dmMuted")}>
                <Icon name="bell" size={12} />
                <span className="nb-dot" />
              </span>
            )}
            {activity && (
              <span className="sb-dm-game" title={activity.name}>
                {activity.name}
              </span>
            )}
            {pinnedDms.includes(d.id) && (
              <span className="sb-pinned" title={t("pinned")} aria-label={t("pinned")}>
                <Icon name="pin" size={11} />
              </span>
            )}
            {d.lastAt && (
              <time className="sb-dm-time" dateTime={d.lastAt}>
                {formatListTime(d.lastAt, now, i18n.language)}
              </time>
            )}
          </div>
          <div className="sb-dm-subrow">
            {inCallWith(d.id) ? (
              <div className="sb-dm-sub sb-dm-call">
                <Icon name="phone" size={12} />
                <span>{i18n.t("ongoingCall", { ns: "call" })}</span>
              </div>
            ) : (
              <div className={"sb-dm-sub" + (unread > 0 ? " unread" : "")}>
                {sub}
              </div>
            )}
            {unread > 0 && <span className={"sb-dm-badge" + (quiet ? " muted" : "")}>{unread}</span>}
          </div>
        </div>
      </div>
    );
  };
  if (showHidden) {
    return (
      <div className="sb-view">
        <button
          type="button"
          className="sb-add-row secondary sb-footer-back"
          onClick={() => setShowHidden(false)}
        >
          <span className="sb-add-row-ico">
            <Icon name="chev-left" size={20} />
          </span>
          <span>{t("hiddenChats")}</span>
        </button>
        <div className="sb-vhead">
          <span className="sb-vhead-label">{t("hiddenChatsHint")}</span>
        </div>
        {shown.map(renderRow)}
        {hasMoreDirects && onLoadMoreDirects && (
          <button type="button" className="sb-load-more" onClick={onLoadMoreDirects}>
            {t("loadMore")}
          </button>
        )}
      </div>
    );
  }
  return (
    <div className="sb-view">
      <button type="button" className="sb-add-row" onClick={onAddFriend}>
        <span className="sb-add-row-ico">
          <Icon name="user-plus" size={20} />
        </span>
        <span>{t("addFriend")}</span>
      </button>
      <button
        type="button"
        className="sb-add-row secondary"
        onClick={onOpenPending}
      >
        <span className="sb-add-row-ico">
          <Icon name="mail" size={20} />
        </span>
        <span>{t("pendingRequests")}</span>
        {pendingCount > 0 && (
          <span className="sb-add-row-badge">{pendingCount}</span>
        )}
      </button>
      {hasHidden && (
        <button type="button" className="sb-add-row secondary sb-hidden-folder" onClick={() => setShowHidden(true)}>
          <span className="sb-add-row-ico">
            <Icon name="eye-off" size={20} />
          </span>
          <span>{t("hiddenChats")}</span>
          {hiddenUnread > 0 && <span className="sb-add-row-badge muted">{hiddenUnread}</span>}
        </button>
      )}
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("directMessages")}</span>
      </div>
      {shown.length === 0 && (
        <div className="sb-empty">
          {t("noDirects")}
        </div>
      )}
      {shown.map(renderRow)}
      {hasMoreDirects && onLoadMoreDirects && (
        <button type="button" className="sb-load-more" onClick={onLoadMoreDirects}>
          {t("loadMore")}
        </button>
      )}
    </div>
  );
};

export default DmList;
