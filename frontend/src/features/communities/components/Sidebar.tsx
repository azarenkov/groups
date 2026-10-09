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
import SidebarFooter from "./sidebar/SidebarFooter";
import SidebarHeader from "./sidebar/SidebarHeader";
import GlobalSearchResults from "../../search/components/GlobalSearchResults";
import type { OpenMessageLink } from "../../chat/lib/messageLink";
import { pinnedFirst, useConversationPinsStore } from "../store/conversationPinsStore";
import { useSidebarPermissions } from "./sidebar/permissions";
import { useSidebarNavigation } from "./sidebar/useSidebarNavigation";
import type { SidebarVoiceMember } from "./sidebar/types";
import { useHotkey } from "../../settings/lib/hotkeys";
import {
  SIDEBAR_COLLAPSED_WIDTH,
  SIDEBAR_MAX_WIDTH,
  useSidebarLayoutStore,
} from "../store/sidebarLayoutStore";

const RESIZE_STEP = 16;

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
  onOpenSettings: () => void;
  onAddFriend: () => void;
  onOpenPending: () => void;
  onGoHome: () => void;
  homeActive: boolean;
  pendingCount: number;
  onCreateCommunity: () => void;
  onJoinCommunity: () => void;
  currentUserId?: string;
  currentUserName: string;
  currentUserStatus: string;
  voiceMembers?: SidebarVoiceMember[];
  voiceMembersByChannel?: Record<string, SidebarVoiceMember[]>;
  searchQuery: string;
  onSearchQueryChange: (value: string) => void;
  unreadByChannel?: Record<string, number>;
  unreadByDm?: Record<string, number>;
  channelToCommunity?: Record<string, string>;
  onSetStatus?: (status: "online" | "offline") => void;
  onLeaveCommunity?: (community: Community) => void;
  onCallDm?: (dm: DirectConversation) => void;
  onOpenSearchResult?: (link: OpenMessageLink) => void;
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
  onOpenSettings,
  onAddFriend,
  onOpenPending,
  onGoHome,
  homeActive,
  pendingCount,
  onCreateCommunity,
  onJoinCommunity,
  currentUserId,
  currentUserName,
  currentUserStatus,
  voiceMembers,
  voiceMembersByChannel,
  searchQuery,
  onSearchQueryChange,
  unreadByChannel = {},
  unreadByDm = {},
  channelToCommunity = {},
  onSetStatus,
  onLeaveCommunity,
  onCallDm,
  onOpenSearchResult,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const { tab, openCommunityId, navState, goHistory, setTab, setOpenCommunityId } =
    useSidebarNavigation({ selectedChannelId, selectedDmId, channelsByCommunity });
  const openCommunity = communities.find((s) => s.id === openCommunityId) ?? null;
  const perms = useSidebarPermissions(currentUserId, openCommunityId);
  const collapsed = useSidebarLayoutStore((s) => s.collapsed);
  const width = useSidebarLayoutStore((s) => s.width);
  const [focusSearchToken, setFocusSearchToken] = React.useState(0);
  const shownWidth = collapsed ? SIDEBAR_COLLAPSED_WIDTH : width;

  React.useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--w-sidebar", `${shownWidth}px`);
    return () => {
      root.style.removeProperty("--w-sidebar");
    };
  }, [shownWidth]);

  const expandAndSearch = () => {
    useSidebarLayoutStore.getState().setCollapsed(false);
    setFocusSearchToken((n) => n + 1);
  };
  useHotkey("quickSwitcher", expandAndSearch);

  const collapse = () => {
    if (searchQuery) onSearchQueryChange("");
    useSidebarLayoutStore.getState().setCollapsed(true);
  };

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
      const layout = useSidebarLayoutStore.getState();
      if (layout.collapsed && searchQuery) onSearchQueryChange("");
      layout.persist();
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
      onDoubleClick={() => (collapsed ? useSidebarLayoutStore.getState().setCollapsed(false) : collapse())}
      onKeyDown={(e) => {
        if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
          e.preventDefault();
          useSidebarLayoutStore.getState().nudge(e.key === "ArrowLeft" ? -RESIZE_STEP : RESIZE_STEP);
          if (useSidebarLayoutStore.getState().collapsed && searchQuery) onSearchQueryChange("");
        } else if (e.key === "Enter") {
          e.preventDefault();
          if (collapsed) useSidebarLayoutStore.getState().setCollapsed(false);
          else collapse();
        }
      }}
    />
  );

  const q = searchQuery.trim().toLowerCase();
  const match = (s: string) => s.toLowerCase().includes(q);
  const isFiltering = q.length > 0;

  const dmPins = useConversationPinsStore((s) => s.dms);
  const communityPins = useConversationPinsStore((s) => s.communities);
  const visibleDirects = pinnedFirst(isFiltering ? directs.filter((d) => match(d.name)) : directs, dmPins);

  const filteredCommunities = pinnedFirst(
    isFiltering
      ? communities.filter((community) => {
          if (match(community.name)) return true;
          const channels = channelsByCommunity[community.id] || [];
          return channels.some((c) => match(c.name));
        })
      : communities,
    communityPins,
  );

  const visibleChannelsFor = (communityId: string) => {
    const channels = channelsByCommunity[communityId] || [];
    if (!isFiltering) return channels;
    const community = communities.find((s) => s.id === communityId);
    if (community && match(community.name)) return channels;
    return channels.filter((c) => match(c.name));
  };

  const settleAfterPick = (target: "dms" | "communities") => {
    if (!isFiltering) return;
    setTab(target);
    onSearchQueryChange("");
  };

  const pickDm = (d: DirectConversation) => {
    onSelectDm(d);
    settleAfterPick("dms");
  };

  const pickCommunity = (community: Community) => {
    if (!expandedCommunityIds.has(community.id)) onToggleCommunity(community.id);
    setOpenCommunityId(community.id);
    onSelectCommunity(community.id);
    settleAfterPick("communities");
  };

  const pickChannel = (community: Community, channel: Channel) => {
    onSelectChannel(community, channel);
    if (!expandedCommunityIds.has(community.id)) onToggleCommunity(community.id);
    setOpenCommunityId(community.id);
    settleAfterPick("communities");
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
      isFiltering={isFiltering}
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
    openCommunity && !isFiltering ? (
      <CommunityPage
        community={openCommunity}
        channels={visibleChannelsFor(openCommunity.id)}
        perms={perms}
        onInviteCommunity={onInviteCommunity}
        onOpenCommunitySettings={onOpenCommunitySettings}
        onAddChannel={onAddChannel}
        renderChannel={renderChannel}
      />
    ) : (
      <CommunityList
        communities={filteredCommunities}
        isFiltering={isFiltering}
        channelsByCommunity={channelsByCommunity}
        voiceMembersByChannel={voiceMembersByChannel}
        unreadFor={communityUnreadFor}
        visibleChannelsFor={visibleChannelsFor}
        onCreateCommunity={onCreateCommunity}
        onJoinCommunity={onJoinCommunity}
        onPick={pickCommunity}
        renderChannel={renderChannel}
        perms={perms}
        currentUserId={currentUserId}
        onInviteCommunity={onInviteCommunity}
        onOpenCommunitySettings={onOpenCommunitySettings}
        onLeaveCommunity={onLeaveCommunity}
      />
    );

  if (collapsed) {
    const otherTab = tab === "dms" ? "communities" : "dms";
    const otherDot = otherTab === "dms" ? totalDmUnread + pendingCount > 0 : totalCommunityUnread > 0;
    const switchLabel = t(otherTab === "dms" ? "showDirects" : "showCommunities");
    return (
      <div className="sb sb-collapsed">
        <div className="sb-rail">
          <button type="button" className="sb-rail-btn" title={switchLabel} aria-label={switchLabel} onClick={() => setTab(otherTab)}>
            <Icon name={otherTab === "dms" ? "message" : "users"} size={17} />
            {otherDot && <span className="sb-rail-dot" />}
          </button>
          <button type="button" className="sb-rail-btn" title={t("addFriend")} aria-label={t("addFriend")} onClick={onAddFriend}>
            <Icon name="user-plus" size={17} />
          </button>
          <button
            type="button"
            className="sb-rail-btn"
            title={t("pendingRequests")}
            aria-label={pendingCount > 0 ? `${t("pendingRequests")}: ${pendingCount}` : t("pendingRequests")}
            onClick={onOpenPending}
          >
            <Icon name="mail" size={17} />
            {pendingCount > 0 && <span className="sb-rail-badge">{pendingCount > 99 ? "99+" : pendingCount}</span>}
          </button>
          <button
            type="button"
            className={"sb-rail-btn" + (homeActive ? " active" : "")}
            title={t("home")}
            aria-label={t("home")}
            onClick={onGoHome}
          >
            <Icon name="home" size={17} />
          </button>
          <button type="button" className="sb-rail-btn" title={t("searchPlaceholder")} aria-label={t("searchPlaceholder")} onClick={expandAndSearch}>
            <Icon name="search" size={16} />
          </button>
        </div>
        <div className="sb-scroll sb-scroll-mini">
          {tab === "dms" ? (
            <DmList
              compact
              directs={pinnedFirst(directs, dmPins)}
              isFiltering={false}
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
              activeCommunityId={openCommunityId}
              communities={pinnedFirst(communities, communityPins)}
              isFiltering={false}
              channelsByCommunity={channelsByCommunity}
              voiceMembersByChannel={voiceMembersByChannel}
              unreadFor={communityUnreadFor}
              visibleChannelsFor={visibleChannelsFor}
              onCreateCommunity={onCreateCommunity}
              onJoinCommunity={onJoinCommunity}
              onPick={pickCommunity}
              renderChannel={renderChannel}
              perms={perms}
              currentUserId={currentUserId}
              onInviteCommunity={onInviteCommunity}
              onOpenCommunitySettings={onOpenCommunitySettings}
              onLeaveCommunity={onLeaveCommunity}
            />
          )}
        </div>
        <div id="voice-dock-slot" className="sb-voice-dock" />
        <button
          type="button"
          className="sb-rail-btn sb-rail-expand"
          title={t("expandSidebar")}
          aria-label={t("expandSidebar")}
          onClick={() => useSidebarLayoutStore.getState().setCollapsed(false)}
        >
          <Icon name="expand" size={15} />
        </button>
        <SidebarFooter
          compact
          currentUserId={currentUserId}
          currentUserName={currentUserName}
          currentUserStatus={currentUserStatus}
          onOpenSettings={onOpenSettings}
          onSetStatus={onSetStatus}
        />
        {resizer}
      </div>
    );
  }

  return (
    <div className="sb">
      <SidebarHeader
        homeActive={homeActive}
        onGoHome={onGoHome}
        navState={navState}
        goHistory={goHistory}
        query={searchQuery}
        onQueryChange={onSearchQueryChange}
        tab={tab}
        onSetTab={setTab}
        openCommunityId={openCommunityId}
        onCloseCommunity={() => setOpenCommunityId(null)}
        dmDotVisible={totalDmUnread + pendingCount > 0}
        communityDotVisible={totalCommunityUnread > 0}
        onCollapse={collapse}
        focusSearchToken={focusSearchToken}
      />

      <div className="sb-scroll">
        {isFiltering ? (
          <>
            {dmList}
            {communityView}
            {onOpenSearchResult && (
              <GlobalSearchResults query={searchQuery} currentUserId={currentUserId} onOpen={onOpenSearchResult} />
            )}
          </>
        ) : tab === "dms" ? (
          dmList
        ) : (
          communityView
        )}
      </div>

      <div id="voice-dock-slot" className="sb-voice-dock" />

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
      <SidebarFooter
        currentUserId={currentUserId}
        currentUserName={currentUserName}
        currentUserStatus={currentUserStatus}
        onOpenSettings={onOpenSettings}
        onSetStatus={onSetStatus}
      />
      {resizer}
    </div>
  );
};

export default Sidebar;
