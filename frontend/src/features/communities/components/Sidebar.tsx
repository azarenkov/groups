import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import type { Community } from "../../../shared/types/community";
import type { Channel, ChannelKind } from "../../../shared/types/channel";
import type { DirectConversation } from "../../dm/types";
import ChannelRow from "./sidebar/ChannelRow";
import DmList from "./sidebar/DmList";
import CommunityList from "./sidebar/CommunityList";
import CommunityPage from "./sidebar/CommunityPage";
import SidebarHeader from "./sidebar/SidebarHeader";
import { pinnedFirst, useConversationPinsStore } from "../store/conversationPinsStore";
import { useSidebarPermissions } from "./sidebar/permissions";
import type { SidebarVoiceMember } from "./sidebar/types";
import {
  SIDEBAR_COLLAPSED_WIDTH,
  SIDEBAR_MAX_WIDTH,
  SIDEBAR_MIN_WIDTH,
  useSidebarLayoutStore,
} from "../store/sidebarLayoutStore";
import { PANELS_CHANGED, announcePanels, clampPanelWidth } from "../../../shared/lib/panelWidth";

const RESIZE_STEP = 16;
const SIDEBAR_CSS_VAR = "--w-sidebar";

interface SidebarProps {
  communities: Community[];
  channelsByCommunity: Record<string, Channel[]>;
  expandedCommunityIds: Set<string>;
  selectedChannelId: string | null;
  joinedChannelId: string | null;
  selectedDmId: string | null;
  directs: DirectConversation[];
  hasMoreDirects?: boolean;
  onLoadMoreDirects?: () => void;
  onToggleCommunity: (communityId: string) => void;
  onSelectCommunity: (communityId: string) => void;
  onSelectChannel: (community: Community, channel: Channel) => void;
  onSelectDm: (dm: DirectConversation) => void;
  onInviteCommunity: (community: Community) => void;
  onOpenCommunitySettings: (community: Community) => void;
  onAddChannel: (community: Community, kind: ChannelKind) => void;
  onDeleteChannel: (community: Community, channel: Channel) => void;
  onAddFriend: () => void;
  onOpenPending: () => void;
  pendingCount: number;
  onCreateCommunity: () => void;
  onJoinCommunity: () => void;
  currentUserId?: string;
  voiceMembers?: SidebarVoiceMember[];
  voiceMembersByChannel?: Record<string, SidebarVoiceMember[]>;
  unreadByChannel?: Record<string, number>;
  unreadByDm?: Record<string, number>;
  channelToCommunity?: Record<string, string>;
  onLeaveCommunity?: (community: Community) => void;
  onCallDm?: (dm: DirectConversation) => void;
  nav: SidebarNav;
}

export interface SidebarNav {
  tab: "dms" | "communities";
  setTab: (tab: "dms" | "communities") => void;
  openCommunityId: string | null;
  setOpenCommunityId: (id: string | null) => void;
}

const Sidebar: React.FC<SidebarProps> = ({
  communities,
  channelsByCommunity,
  expandedCommunityIds,
  selectedChannelId,
  joinedChannelId,
  selectedDmId,
  directs,
  hasMoreDirects,
  onLoadMoreDirects,
  onToggleCommunity,
  onSelectCommunity,
  onSelectChannel,
  onSelectDm,
  onInviteCommunity,
  onOpenCommunitySettings,
  onAddChannel,
  onDeleteChannel,
  onAddFriend,
  onOpenPending,
  pendingCount,
  onCreateCommunity,
  onJoinCommunity,
  currentUserId,
  voiceMembers,
  voiceMembersByChannel,
  unreadByChannel = {},
  unreadByDm = {},
  channelToCommunity = {},
  onLeaveCommunity,
  onCallDm,
  nav,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const { tab, openCommunityId, setTab, setOpenCommunityId } = nav;
  const openCommunity = communities.find((s) => s.id === openCommunityId) ?? null;
  const perms = useSidebarPermissions(currentUserId, openCommunityId);
  const collapsed = useSidebarLayoutStore((s) => s.collapsed);
  const width = useSidebarLayoutStore((s) => s.width);
  const [roomyWidth, setRoomyWidth] = React.useState(width);
  const [flyoutId, setFlyoutId] = React.useState<string | null>(null);
  const flyoutRef = React.useRef<HTMLDivElement>(null);
  const showRail = collapsed;
  const shownWidth = showRail ? SIDEBAR_COLLAPSED_WIDTH : roomyWidth;

  React.useEffect(() => {
    const fit = () =>
      setRoomyWidth(
        clampPanelWidth(
          width,
          { min: SIDEBAR_MIN_WIDTH, max: SIDEBAR_MAX_WIDTH },
          window.innerWidth,
          document.querySelector(".members-panel")?.getBoundingClientRect().width ?? 0,
        ),
      );
    const onPanels = (e: Event) => {
      if ((e as CustomEvent<string>).detail !== SIDEBAR_CSS_VAR) fit();
    };
    fit();
    window.addEventListener("resize", fit);
    window.addEventListener(PANELS_CHANGED, onPanels);
    return () => {
      window.removeEventListener("resize", fit);
      window.removeEventListener(PANELS_CHANGED, onPanels);
    };
  }, [width]);

  React.useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty(SIDEBAR_CSS_VAR, `${shownWidth}px`);
    announcePanels(SIDEBAR_CSS_VAR);
  }, [shownWidth]);

  React.useEffect(
    () => () => {
      document.documentElement.style.removeProperty(SIDEBAR_CSS_VAR);
      requestAnimationFrame(() => announcePanels(SIDEBAR_CSS_VAR));
    },
    [],
  );

  const collapse = () => {
    useSidebarLayoutStore.getState().setCollapsed(true);
  };
  const expand = () => useSidebarLayoutStore.getState().setCollapsed(false);

  const startResize = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const handle = e.currentTarget;
    const left = (handle.parentElement ?? handle).getBoundingClientRect().left;
    handle.setPointerCapture?.(e.pointerId);
    document.body.classList.add("sb-resizing");
    const move = (ev: PointerEvent) => useSidebarLayoutStore.getState().dragTo(ev.clientX - left);
    const stop = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", stop);
      handle.removeEventListener("pointercancel", stop);
      document.body.classList.remove("sb-resizing");
      handle.blur();
      useSidebarLayoutStore.getState().persist();
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", stop);
    handle.addEventListener("pointercancel", stop);
  };

  const resizer = (
    <div
      className="sb-resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label={t("resizeSidebar")}
      aria-valuemin={SIDEBAR_COLLAPSED_WIDTH}
      aria-valuemax={SIDEBAR_MAX_WIDTH}
      aria-valuenow={shownWidth}
      tabIndex={0}
      onPointerDown={startResize}
      onDoubleClick={(e) => {
        e.currentTarget.blur();
        if (showRail) expand();
        else collapse();
      }}
      onKeyDown={(e) => {
        if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
          e.preventDefault();
          useSidebarLayoutStore.getState().nudge(e.key === "ArrowLeft" ? -RESIZE_STEP : RESIZE_STEP);
        } else if (e.key === "Enter") {
          e.preventDefault();
          if (showRail) expand();
          else collapse();
        }
      }}
    />
  );

  const dmPins = useConversationPinsStore((s) => s.dms);
  const communityPins = useConversationPinsStore((s) => s.communities);
  const visibleDirects = pinnedFirst(directs, dmPins);
  const orderedCommunities = pinnedFirst(communities, communityPins);
  const channelsOf = (communityId: string) => channelsByCommunity[communityId] || [];

  const pickDm = (d: DirectConversation) => {
    onSelectDm(d);
  };

  const pickCommunity = (community: Community) => {
    if (!expandedCommunityIds.has(community.id)) onToggleCommunity(community.id);
    setOpenCommunityId(community.id);
    onSelectCommunity(community.id);
  };

  const pickChannel = (community: Community, channel: Channel) => {
    setFlyoutId(null);
    onSelectChannel(community, channel);
    if (!expandedCommunityIds.has(community.id)) onToggleCommunity(community.id);
    setOpenCommunityId(community.id);
  };

  const totalDmUnread = Object.values(unreadByDm).reduce((a, b) => a + b, 0);
  const totalCommunityUnread = Object.values(unreadByChannel).reduce(
    (a, b) => a + b,
    0,
  );

  const communityUnreadFor = (communityId: string) => {
    const all = channelsByCommunity[communityId] || [];
    const localSum = all
      .filter((c) => c.kind === "text")
      .reduce((sum, c) => sum + (unreadByChannel[c.id] ?? 0), 0);
    let fallbackSum = 0;
    for (const [cid, count] of Object.entries(unreadByChannel)) {
      if (channelToCommunity[cid] === communityId) fallbackSum += count;
    }
    return Math.max(localSum, fallbackSum);
  };

  const renderChannel = (community: Community, channel: Channel) => (
    <ChannelRow
      key={channel.id}
      community={community}
      channel={channel}
      perms={perms}
      selectedChannelId={selectedChannelId}
      joinedChannelId={joinedChannelId}
      unreadByChannel={unreadByChannel}
      voiceMembers={voiceMembers}
      voiceMembersByChannel={voiceMembersByChannel}
      currentUserId={currentUserId}
      onPick={pickChannel}
      onDeleteChannel={onDeleteChannel}
    />
  );

  const dmList = (
    <DmList
      directs={visibleDirects}
      selectedDmId={selectedDmId}
      unreadByDm={unreadByDm}
      pendingCount={pendingCount}
      hasMoreDirects={hasMoreDirects}
      onLoadMoreDirects={onLoadMoreDirects}
      onAddFriend={onAddFriend}
      onOpenPending={onOpenPending}
      onPick={pickDm}
      onCall={onCallDm}
    />
  );

  const communityView =
    openCommunity ? (
      <CommunityPage
        community={openCommunity}
        channels={channelsOf(openCommunity.id)}
        perms={perms}
        onInviteCommunity={onInviteCommunity}
        onOpenCommunitySettings={onOpenCommunitySettings}
        onAddChannel={onAddChannel}
        renderChannel={renderChannel}
      />
    ) : (
      <CommunityList
        communities={orderedCommunities}
        channelsByCommunity={channelsByCommunity}
        voiceMembersByChannel={voiceMembersByChannel}
        unreadFor={communityUnreadFor}
        onCreateCommunity={onCreateCommunity}
        onJoinCommunity={onJoinCommunity}
        onPick={pickCommunity}
        perms={perms}
        currentUserId={currentUserId}
        onInviteCommunity={onInviteCommunity}
        onOpenCommunitySettings={onOpenCommunitySettings}
        onLeaveCommunity={onLeaveCommunity}
      />
    );

  const flyoutCommunity = showRail ? (communities.find((c) => c.id === flyoutId) ?? null) : null;

  React.useEffect(() => {
    if (!flyoutCommunity) return;
    const onDoc = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (flyoutRef.current?.contains(target)) return;
      if (target.closest?.(`[data-community-id="${flyoutCommunity.id}"]`)) return;
      setFlyoutId(null);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setFlyoutId(null);
    };
    document.addEventListener("mousedown", onDoc);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      window.removeEventListener("keydown", onKey);
    };
  }, [flyoutCommunity]);

  const pickFromRail = (community: Community) => {
    if (flyoutId === community.id) {
      setFlyoutId(null);
      return;
    }
    pickCommunity(community);
    setFlyoutId(community.id);
  };

  if (showRail) {
    return (
      <div className="sb sb-collapsed">
        <div className="sb-rail">
          <button type="button" className="sb-rail-btn sb-rail-open" title={t("expandSidebar")} aria-label={t("expandSidebar")} onClick={expand}>
            <Icon name="panel-open" size={20} />
          </button>
          <div className="sb-rail-switch" role="tablist" aria-label={t("switchView")}>
            <button
              type="button"
              role="tab"
              aria-selected={tab === "dms"}
              className={"sb-rail-switch-btn" + (tab === "dms" ? " active" : "")}
              title={t("showDirects")}
              aria-label={t("showDirects")}
              onClick={() => setTab("dms")}
            >
              <Icon name="message" size={17} />
              {tab !== "dms" && totalDmUnread + pendingCount > 0 && <span className="sb-rail-dot" />}
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={tab === "communities"}
              className={"sb-rail-switch-btn" + (tab === "communities" ? " active" : "")}
              title={t("showCommunities")}
              aria-label={t("showCommunities")}
              onClick={() => setTab("communities")}
            >
              <Icon name="users" size={17} />
              {tab !== "communities" && totalCommunityUnread > 0 && <span className="sb-rail-dot" />}
            </button>
          </div>
        </div>
        <div className="sb-scroll sb-scroll-mini">
          {tab === "dms" ? (
            <DmList
              compact
              directs={visibleDirects}
              selectedDmId={selectedDmId}
              unreadByDm={unreadByDm}
              pendingCount={pendingCount}
              hasMoreDirects={hasMoreDirects}
              onLoadMoreDirects={onLoadMoreDirects}
              onAddFriend={onAddFriend}
              onOpenPending={onOpenPending}
              onPick={pickDm}
              onCall={onCallDm}
            />
          ) : (
            <CommunityList
              compact
              activeCommunityId={flyoutId ?? openCommunityId}
              communities={orderedCommunities}
              channelsByCommunity={channelsByCommunity}
              voiceMembersByChannel={voiceMembersByChannel}
              unreadFor={communityUnreadFor}
              onCreateCommunity={onCreateCommunity}
              onJoinCommunity={onJoinCommunity}
              onPick={pickFromRail}
              perms={perms}
              currentUserId={currentUserId}
              onInviteCommunity={onInviteCommunity}
              onOpenCommunitySettings={onOpenCommunitySettings}
              onLeaveCommunity={onLeaveCommunity}
            />
          )}
        </div>
        {flyoutCommunity && (
          <div className="sb-flyout" role="dialog" aria-label={flyoutCommunity.name} ref={flyoutRef}>
            <CommunityPage
              community={flyoutCommunity}
              channels={channelsOf(flyoutCommunity.id)}
              perms={perms}
              onInviteCommunity={onInviteCommunity}
              onOpenCommunitySettings={onOpenCommunitySettings}
              onAddChannel={onAddChannel}
              renderChannel={renderChannel}
            />
          </div>
        )}
        {resizer}
      </div>
    );
  }

  return (
    <div className="sb">
      <SidebarHeader
        tab={tab}
        onSetTab={setTab}
        openCommunityId={openCommunityId}
        onCloseCommunity={() => setOpenCommunityId(null)}
        dmDotVisible={totalDmUnread + pendingCount > 0}
        communityDotVisible={totalCommunityUnread > 0}
        onCollapse={collapse}
      />

      <div className="sb-scroll">
        {tab === "dms" ? (
          dmList
        ) : (
          communityView
        )}
      </div>

      {openCommunityId && (
        <button
          type="button"
          className="sb-add-row secondary sb-footer-back"
          onClick={() => setOpenCommunityId(null)}
        >
          <span className="sb-add-row-ico">
            <Icon name="chev-left" size={13} />
          </span>
          <span>{t("backToCommunities")}</span>
        </button>
      )}
      {resizer}
    </div>
  );
};

export default Sidebar;
