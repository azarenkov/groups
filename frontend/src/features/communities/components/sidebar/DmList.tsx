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
import { dmLevelOf, useNotificationPrefsStore } from "../../../settings/store/notificationPrefsStore";
import { MAX_PINNED_CONVERSATIONS, useConversationPinsStore } from "../../store/conversationPinsStore";
import { useNotificationSettingsStore } from "../../../settings/store/notificationSettingsStore";
import {
  openContextMenu,
  openContextMenuAt,
} from "../../../../shared/ui/contextMenu/contextMenuStore";
import type { MenuEntry } from "../../../../shared/ui/contextMenu/contextMenuStore";
import { useLongPress } from "../../../../shared/ui/contextMenu/useLongPress";

interface DmListProps {
  directs: DirectConversation[];
  isFiltering: boolean;
  selectedDmId: string | null;
  unreadByDm: Record<string, number>;
  pendingCount: number;
  hasMoreDirects?: boolean;
  onLoadMoreDirects?: () => void;
  onAddFriend: () => void;
  onOpenPending: () => void;
  onPick: (dm: DirectConversation) => void;
  onCall?: (dm: DirectConversation) => void;
}

const DmList: React.FC<DmListProps> = ({
  directs,
  isFiltering,
  selectedDmId,
  unreadByDm,
  pendingCount,
  hasMoreDirects,
  onLoadMoreDirects,
  onAddFriend,
  onOpenPending,
  onPick,
  onCall,
}) => {
  const { t, i18n } = useTranslation(["sidebar", "common"]);
  const activities = useActivityStore((s) => s.byUser);
  const dmLevels = useNotificationPrefsStore((s) => s.dmLevels);
  const pinnedDms = useConversationPinsStore((s) => s.dms);
  const accessiblePins = () => pinnedDms.filter((id) => !isBlockedEitherWay(id));
  const globalMentions = useNotificationSettingsStore((s) => s.mentions);
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
      disabled: !pinnedDms.includes(d.id) && accessiblePins().length >= MAX_PINNED_CONVERSATIONS,
      onSelect: () => {
        void useConversationPinsStore
          .getState()
          .setPinned("dms", d.id, !pinnedDms.includes(d.id), (pinnedId) => !isBlockedEitherWay(pinnedId))
          .catch((e) => console.warn("[DmList] pin failed:", e));
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
  ];
  const dmLongPress = useLongPress((point, el) => {
    const dm = directs.find((d) => d.id === el.dataset.dmId);
    return !!dm && openContextMenu(point, dmMenu(dm));
  });
  return (
    <div className="sb-view">
      {!isFiltering && (
        <>
          <button type="button" className="sb-add-row" onClick={onAddFriend}>
            <span className="sb-add-row-ico">
              <Icon name="user-plus" size={13} />
            </span>
            <span>{t("addFriend")}</span>
          </button>
          <button
            type="button"
            className="sb-add-row secondary"
            onClick={onOpenPending}
          >
            <span className="sb-add-row-ico">
              <Icon name="mail" size={13} />
            </span>
            <span>{t("pendingRequests")}</span>
            {pendingCount > 0 && (
              <span className="sb-add-row-badge">{pendingCount}</span>
            )}
          </button>
        </>
      )}
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("directMessages")}</span>
      </div>
      {directs.length === 0 && (
        <div className="sb-empty">
          {isFiltering ? t("noDirectMatches") : t("noDirects")}
        </div>
      )}
      {directs.map((d) => {
        const unread = unreadByDm[d.id] ?? 0;
        const activity = d.online ? activities[d.id] : undefined;
        const muted = dmLevelOf(dmLevels, d.id, globalMentions) === "none";
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
                <div className={"sb-dm-sub" + (unread > 0 ? " unread" : "")}>
                  {sub}
                </div>
                {unread > 0 && <span className={"sb-dm-badge" + (muted ? " muted" : "")}>{unread}</span>}
              </div>
            </div>
          </div>
        );
      })}
      {hasMoreDirects && onLoadMoreDirects && (
        <button type="button" className="sb-load-more" onClick={onLoadMoreDirects}>
          {t("loadMore")}
        </button>
      )}
    </div>
  );
};

export default DmList;
