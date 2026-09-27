import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import type { DirectConversation } from "../../../dm/types";
import { useActivityStore } from "../../../social/store/activityStore";
import UserBadge from "../../../subscription/components/UserBadge";
import { formatListTime } from "../../../dm/lib/directOrder";
import { useProfileModalStore } from "../../../social/store/profileModalStore";
import { isBlockedEitherWay } from "../../../social/store/blockStore";
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
  const now = new Date();
  const dmMenu = (d: DirectConversation): MenuEntry[] => [
    { kind: "item", id: "message", label: t("memberMessage"), icon: "message", onSelect: () => onPick(d) },
    ...(onCall && !isBlockedEitherWay(d.id)
      ? [{ kind: "item" as const, id: "call", label: t("callFriend"), icon: "phone" as const, onSelect: () => onCall(d) }]
      : []),
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
                {activity && (
                  <span className="sb-dm-game" title={activity.name}>
                    {activity.name}
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
                {unread > 0 && <span className="sb-dm-badge">{unread}</span>}
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
