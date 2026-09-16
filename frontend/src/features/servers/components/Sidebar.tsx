import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import type { Server } from "../../../shared/types/server";
import type { Channel, ChannelKind } from "../../../shared/types/channel";
import type { DirectConversation } from "../../dm/types";
import ChannelRow from "./sidebar/ChannelRow";
import DmList from "./sidebar/DmList";
import ServerList from "./sidebar/ServerList";
import ServerPage from "./sidebar/ServerPage";
import SidebarFooter from "./sidebar/SidebarFooter";
import SidebarHeader from "./sidebar/SidebarHeader";
import { useSidebarPermissions } from "./sidebar/permissions";
import { useSidebarNavigation } from "./sidebar/useSidebarNavigation";
import type { SidebarVoiceMember } from "./sidebar/types";

interface SidebarProps {
  servers: Server[];
  channelsByServer: Record<string, Channel[]>;
  expandedServerIds: Set<string>;
  selectedChannelId: string | null;
  joinedChannelId: string | null;
  selectedDmId: string | null;
  directs: DirectConversation[];
  hasMoreDirects?: boolean;
  onLoadMoreDirects?: () => void;
  onToggleServer: (serverId: string) => void;
  onSelectServer: (serverId: string) => void;
  onSelectChannel: (server: Server, channel: Channel) => void;
  onSelectDm: (dm: DirectConversation) => void;
  onInviteServer: (server: Server) => void;
  onOpenServerSettings: (server: Server) => void;
  onAddChannel: (server: Server, kind: ChannelKind) => void;
  onDeleteChannel: (server: Server, channel: Channel) => void;
  onOpenSettings: () => void;
  onAddFriend: () => void;
  onOpenPending: () => void;
  onGoHome: () => void;
  homeActive: boolean;
  pendingCount: number;
  onCreateServer: () => void;
  onJoinServer: () => void;
  currentUserId?: string;
  currentUserName: string;
  currentUserStatus: string;
  voiceMembers?: SidebarVoiceMember[];
  voiceMembersByChannel?: Record<string, SidebarVoiceMember[]>;
  searchQuery: string;
  onSearchQueryChange: (value: string) => void;
  unreadByChannel?: Record<string, number>;
  unreadByDm?: Record<string, number>;
  channelToServer?: Record<string, string>;
  onSetStatus?: (status: "online" | "offline") => void;
}

const Sidebar: React.FC<SidebarProps> = ({
  servers,
  channelsByServer,
  expandedServerIds,
  selectedChannelId,
  joinedChannelId,
  selectedDmId,
  directs,
  hasMoreDirects,
  onLoadMoreDirects,
  onToggleServer,
  onSelectServer,
  onSelectChannel,
  onSelectDm,
  onInviteServer,
  onOpenServerSettings,
  onAddChannel,
  onDeleteChannel,
  onOpenSettings,
  onAddFriend,
  onOpenPending,
  onGoHome,
  homeActive,
  pendingCount,
  onCreateServer,
  onJoinServer,
  currentUserId,
  currentUserName,
  currentUserStatus,
  voiceMembers,
  voiceMembersByChannel,
  searchQuery,
  onSearchQueryChange,
  unreadByChannel = {},
  unreadByDm = {},
  channelToServer = {},
  onSetStatus,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const { tab, openServerId, navState, goHistory, setTab, setOpenServerId } =
    useSidebarNavigation({ selectedChannelId, selectedDmId, channelsByServer });
  const openServer = servers.find((s) => s.id === openServerId) ?? null;
  const perms = useSidebarPermissions(currentUserId, openServerId);

  const q = searchQuery.trim().toLowerCase();
  const match = (s: string) => s.toLowerCase().includes(q);
  const isFiltering = q.length > 0;

  const visibleDirects = isFiltering
    ? directs.filter((d) => match(d.name))
    : directs;

  const filteredServers = isFiltering
    ? servers.filter((server) => {
        if (match(server.name)) return true;
        const channels = channelsByServer[server.id] || [];
        return channels.some((c) => match(c.name));
      })
    : servers;

  const visibleChannelsFor = (serverId: string) => {
    const channels = channelsByServer[serverId] || [];
    if (!isFiltering) return channels;
    const server = servers.find((s) => s.id === serverId);
    if (server && match(server.name)) return channels;
    return channels.filter((c) => match(c.name));
  };

  const settleAfterPick = (target: "dms" | "servers") => {
    if (!isFiltering) return;
    setTab(target);
    onSearchQueryChange("");
  };

  const pickDm = (d: DirectConversation) => {
    onSelectDm(d);
    settleAfterPick("dms");
  };

  const pickServer = (server: Server) => {
    if (!expandedServerIds.has(server.id)) onToggleServer(server.id);
    setOpenServerId(server.id);
    onSelectServer(server.id);
    settleAfterPick("servers");
  };

  const pickChannel = (server: Server, channel: Channel) => {
    onSelectChannel(server, channel);
    if (!expandedServerIds.has(server.id)) onToggleServer(server.id);
    setOpenServerId(server.id);
    settleAfterPick("servers");
  };

  const totalDmUnread = Object.values(unreadByDm).reduce((a, b) => a + b, 0);
  const totalServerUnread = Object.values(unreadByChannel).reduce(
    (a, b) => a + b,
    0,
  );

  const serverUnreadFor = (serverId: string) => {
    const all = channelsByServer[serverId] || [];
    const localSum = all
      .filter((c) => c.kind === "text")
      .reduce((sum, c) => sum + (unreadByChannel[c.id] ?? 0), 0);
    let fallbackSum = 0;
    for (const [cid, count] of Object.entries(unreadByChannel)) {
      if (channelToServer[cid] === serverId) fallbackSum += count;
    }
    return Math.max(localSum, fallbackSum);
  };

  const renderChannel = (server: Server, channel: Channel) => (
    <ChannelRow
      key={channel.id}
      server={server}
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
    />
  );

  const serverView =
    openServer && !isFiltering ? (
      <ServerPage
        server={openServer}
        channels={visibleChannelsFor(openServer.id)}
        perms={perms}
        onInviteServer={onInviteServer}
        onOpenServerSettings={onOpenServerSettings}
        onAddChannel={onAddChannel}
        renderChannel={renderChannel}
      />
    ) : (
      <ServerList
        servers={filteredServers}
        isFiltering={isFiltering}
        channelsByServer={channelsByServer}
        voiceMembersByChannel={voiceMembersByChannel}
        unreadFor={serverUnreadFor}
        visibleChannelsFor={visibleChannelsFor}
        onCreateServer={onCreateServer}
        onJoinServer={onJoinServer}
        onPick={pickServer}
        renderChannel={renderChannel}
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
        openServerId={openServerId}
        onCloseServer={() => setOpenServerId(null)}
        dmDotVisible={totalDmUnread + pendingCount > 0}
        serverDotVisible={totalServerUnread > 0}
      />

      <div className="sb-scroll">
        {isFiltering ? (
          <>
            {dmList}
            {serverView}
          </>
        ) : tab === "dms" ? (
          dmList
        ) : (
          serverView
        )}
      </div>

      <div id="voice-dock-slot" className="sb-voice-dock" />

      {openServerId && (
        <button
          type="button"
          className="sb-add-row secondary sb-footer-back"
          onClick={() => setOpenServerId(null)}
        >
          <span className="sb-add-row-ico">
            <Icon name="chev-left" size={13} />
          </span>
          <span>{t("backToServers")}</span>
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
