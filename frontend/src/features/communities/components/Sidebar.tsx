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
import { useSidebarPermissions } from "./sidebar/permissions";
import { useSidebarNavigation } from "./sidebar/useSidebarNavigation";
import type { SidebarVoiceMember } from "./sidebar/types";

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

  const q = searchQuery.trim().toLowerCase();
  const match = (s: string) => s.toLowerCase().includes(q);
  const isFiltering = q.length > 0;

  const visibleDirects = isFiltering
    ? directs.filter((d) => match(d.name))
    : directs;

  const filteredCommunities = isFiltering
    ? communities.filter((community) => {
        if (match(community.name)) return true;
        const channels = channelsByCommunity[community.id] || [];
        return channels.some((c) => match(c.name));
      })
    : communities;

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
    </div>
  );
};

export default Sidebar;
