import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { CommunityLogo } from "../../../../shared/ui/Avatar";
import { colorForId } from "../../../../shared/lib/avatar";
import type { Community } from "../../../../shared/types/community";
import type { Channel } from "../../../../shared/types/channel";
import type { SidebarVoiceMember } from "./types";
import type { SidebarPermissions } from "./permissions";
import {
  openContextMenu,
  openContextMenuAt,
} from "../../../../shared/ui/contextMenu/contextMenuStore";
import type { MenuEntry } from "../../../../shared/ui/contextMenu/contextMenuStore";
import { MAX_PINNED_CONVERSATIONS, useConversationPinsStore } from "../../store/conversationPinsStore";
import { useLongPress } from "../../../../shared/ui/contextMenu/useLongPress";
import { useRolesStore } from "../../store/rolesStore";

interface CommunityListProps {
  communities: Community[];
  channelsByCommunity: Record<string, Channel[]>;
  voiceMembersByChannel?: Record<string, SidebarVoiceMember[]>;
  unreadFor: (communityId: string) => number;
  onCreateCommunity: () => void;
  onJoinCommunity: () => void;
  onPick: (community: Community) => void;
  perms: SidebarPermissions;
  currentUserId?: string;
  onInviteCommunity: (community: Community) => void;
  onOpenCommunitySettings: (community: Community) => void;
  onLeaveCommunity?: (community: Community) => void;
  compact?: boolean;
  activeCommunityId?: string | null;
}

const CommunityList: React.FC<CommunityListProps> = ({
  communities,
  channelsByCommunity,
  voiceMembersByChannel,
  unreadFor,
  onCreateCommunity,
  onJoinCommunity,
  onPick,
  perms,
  currentUserId,
  onInviteCommunity,
  onOpenCommunitySettings,
  onLeaveCommunity,
  compact,
  activeCommunityId,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  React.useEffect(() => {
    const roles = useRolesStore.getState();
    for (const community of communities) {
      if (!currentUserId || community.ownerId === currentUserId) continue;
      if (roles.byCommunity[community.id] || roles.isTracked(community.id)) continue;
      void roles.fetch(community.id).catch(() => {});
    }
  }, [communities, currentUserId]);
  const pinnedCommunities = useConversationPinsStore((s) => s.communities);
  const pinsLoaded = useConversationPinsStore((s) => s.loaded);
  const communityMenu = (community: Community): MenuEntry[] => {
    const owner = !!currentUserId && community.ownerId === currentUserId;
    const isPinned = pinnedCommunities.includes(community.id);
    return [
      { kind: "item", id: "open", label: t("openCommunity"), icon: "chev", onSelect: () => onPick(community) },
      {
        kind: "item",
        id: "pin",
        label: t(isPinned ? "unpinCommunity" : "pinCommunity"),
        icon: "pin",
        disabled: !pinsLoaded || (!isPinned && pinnedCommunities.length >= MAX_PINNED_CONVERSATIONS),
        onSelect: () => {
          void useConversationPinsStore
            .getState()
            .setPinned("communities", community.id, !isPinned)
            .catch((e) => console.warn("[CommunityList] pin failed:", e));
        },
      },
      ...(perms.canInviteIn(community)
        ? [
            {
              kind: "item" as const,
              id: "invite",
              label: t("inviteFriends"),
              icon: "user-plus" as const,
              onSelect: () => onInviteCommunity(community),
            },
          ]
        : []),
      {
        kind: "item",
        id: "settings",
        label: t("communitySettings"),
        icon: "settings",
        onSelect: () => onOpenCommunitySettings(community),
      },
      ...(onLeaveCommunity
        ? [
            { kind: "separator" as const, id: "s1" },
            {
              kind: "item" as const,
              id: "leave",
              label: owner ? t("deleteCommunity") : t("leaveCommunity"),
              icon: (owner ? "trash" : "logout") as "trash" | "logout",
              danger: true,
              onSelect: () => onLeaveCommunity(community),
            },
          ]
        : []),
    ];
  };
  const communityLongPress = useLongPress((point, el) => {
    const community = communities.find((c) => c.id === el.dataset.communityId);
    return !!community && openContextMenu(point, communityMenu(community));
  });
  if (compact) {
    return (
      <div className="sb-mini-list" role="list" aria-label={t("communities")}>
        {communities.map((community) => {
          const unread = unreadFor(community.id);
          const voice = (channelsByCommunity[community.id] || []).some(
            (c) => c.kind === "voice" && (voiceMembersByChannel?.[c.id]?.length ?? 0) > 0,
          );
          return (
            <button
              key={community.id}
              type="button"
              role="listitem"
              className={"sb-mini sb-mini-community" + (activeCommunityId === community.id ? " active" : "")}
              data-community-id={community.id}
              title={community.name}
              aria-label={unread > 0 ? t("unreadIn", { name: community.name, count: unread }) : community.name}
              aria-current={activeCommunityId === community.id ? "true" : undefined}
              {...communityLongPress}
              onClick={() => onPick(community)}
              onContextMenu={(e) => openContextMenu(e, communityMenu(community))}
              onKeyDown={(e) => {
                if (e.key === "ContextMenu" || (e.key === "F10" && e.shiftKey)) {
                  e.preventDefault();
                  openContextMenuAt(e.currentTarget, communityMenu(community));
                }
              }}
            >
              <CommunityLogo
                communityId={community.id}
                name={community.name}
                className="sb-mini-ava"
                style={{ background: community.accentColor ?? colorForId(community.id) }}
              />
              {voice && <span className="sb-mini-online" />}
              {unread > 0 && <span className="sb-mini-badge">{unread > 99 ? "99+" : unread}</span>}
            </button>
          );
        })}
      </div>
    );
  }
  return (
    <div className="sb-view">
      <button type="button" className="sb-add-row" onClick={onCreateCommunity}>
        <span className="sb-add-row-ico">
          <Icon name="plus" size={13} />
        </span>
        <span>{t("createCommunity")}</span>
      </button>
      <button
        type="button"
        className="sb-add-row secondary"
        onClick={onJoinCommunity}
      >
        <span className="sb-add-row-ico">
          <Icon name="logout" size={13} />
        </span>
        <span>{t("joinCommunity")}</span>
      </button>
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("communities")}</span>
      </div>
      {communities.length === 0 && (
        <div className="sb-empty">
          {t("noCommunities")}
        </div>
      )}
      {communities.map((community) => {
        const accent = community.accentColor ?? colorForId(community.id);
        const allCommunityChannels = channelsByCommunity[community.id] || [];
        const communityUnread = unreadFor(community.id);
        const hasVoiceActivity = allCommunityChannels.some(
          (c) =>
            c.kind === "voice" &&
            (voiceMembersByChannel?.[c.id]?.length ?? 0) > 0,
        );
        return (
          <div key={community.id}>
            <div
              role="button"
              tabIndex={0}
              className="sb-srv"
              data-community-id={community.id}
              {...communityLongPress}
              onClick={() => onPick(community)}
              onContextMenu={(e) => openContextMenu(e, communityMenu(community))}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onPick(community);
                } else if (e.key === "ContextMenu" || (e.key === "F10" && e.shiftKey)) {
                  e.preventDefault();
                  openContextMenuAt(e.currentTarget, communityMenu(community));
                }
              }}
            >
              <CommunityLogo
                communityId={community.id}
                name={community.name}
                className="sb-srv-ava"
                style={{ background: accent }}
              />
              <span className="sb-srv-name">{community.name}</span>
              {hasVoiceActivity && (
                <span
                  className="sb-community-voice-dot"
                  title="Someone is in voice"
                  aria-label="Voice activity"
                />
              )}
              {pinnedCommunities.includes(community.id) && (
                <span className="sb-pinned" title={t("pinned")} aria-label={t("pinned")}>
                  <Icon name="pin" size={11} />
                </span>
              )}
              {communityUnread > 0 && (
                <span className="sb-community-unread">{communityUnread}</span>
              )}
              <Icon name="chev" size={13} className="sb-srv-chev collapsed" />
            </div>
          </div>
        );
      })}
    </div>
  );
};

export default CommunityList;
