import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { CommunityLogo } from "../../../../shared/ui/Avatar";
import { colorForId } from "../../../../shared/lib/avatar";
import type { Community } from "../../../../shared/types/community";
import type { Channel, ChannelKind } from "../../../../shared/types/channel";
import type { SidebarPermissions } from "./permissions";

interface CommunityPageProps {
  community: Community;
  channels: Channel[];
  perms: SidebarPermissions;
  onInviteCommunity: (community: Community) => void;
  onOpenCommunitySettings: (community: Community) => void;
  onAddChannel: (community: Community, kind: ChannelKind) => void;
  renderChannel: (community: Community, channel: Channel) => React.ReactNode;
}

const CommunityPage: React.FC<CommunityPageProps> = ({
  community,
  channels,
  perms,
  onInviteCommunity,
  onOpenCommunitySettings,
  onAddChannel,
  renderChannel,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  const textChannels = [...channels.filter((c) => c.kind === "text")].sort(
    (a, b) => Number(!!b.isDefault) - Number(!!a.isDefault),
  );
  const voiceChannels = channels.filter((c) => c.kind === "voice");
  const accent = community.accentColor ?? colorForId(community.id);
  return (
    <div className="sb-view sb-community-page">
      <div className="sb-community-head">
        <CommunityLogo
          communityId={community.id}
          name={community.name}
          className="sb-srv-ava"
          style={{ background: accent }}
        />
        <span className="sb-srv-name">{community.name}</span>
        {perms.canInviteIn(community) && (
          <button
            type="button"
            className="sb-section-add"
            title={t("inviteFriends")}
            onClick={() => onInviteCommunity(community)}
          >
            <Icon name="user-plus" size={14} />
          </button>
        )}
        <button
          type="button"
          className="sb-section-add"
          title={t("communitySettings")}
          onClick={() => onOpenCommunitySettings(community)}
        >
          <Icon name="settings" size={14} />
        </button>
      </div>
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("textChannels")}</span>
        {perms.canManageChannelsIn(community) && (
          <button
            type="button"
            className="sb-vhead-add"
            title={t("addChannel")}
            onClick={() => onAddChannel(community, "text")}
          >
            <Icon name="plus" size={14} />
          </button>
        )}
      </div>
      <div className="sb-srv-channels">
        {textChannels.map((c) => renderChannel(community, c))}
      </div>
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("voiceChannels")}</span>
        {perms.canManageChannelsIn(community) && (
          <button
            type="button"
            className="sb-vhead-add"
            title={t("addChannel")}
            onClick={() => onAddChannel(community, "voice")}
          >
            <Icon name="plus" size={14} />
          </button>
        )}
      </div>
      <div className="sb-srv-channels">
        {voiceChannels.map((c) => renderChannel(community, c))}
      </div>
    </div>
  );
};

export default CommunityPage;
