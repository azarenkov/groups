import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { CommunityLogo } from "../../../../shared/ui/Avatar";
import { colorForId } from "../../../../shared/lib/avatar";
import type { Community } from "../../../../shared/types/community";
import type { Channel } from "../../../../shared/types/channel";
import type { SidebarVoiceMember } from "./types";

interface CommunityListProps {
  communities: Community[];
  isFiltering: boolean;
  channelsByCommunity: Record<string, Channel[]>;
  voiceMembersByChannel?: Record<string, SidebarVoiceMember[]>;
  unreadFor: (communityId: string) => number;
  visibleChannelsFor: (communityId: string) => Channel[];
  onCreateCommunity: () => void;
  onJoinCommunity: () => void;
  onPick: (community: Community) => void;
  renderChannel: (community: Community, channel: Channel) => React.ReactNode;
}

const CommunityList: React.FC<CommunityListProps> = ({
  communities,
  isFiltering,
  channelsByCommunity,
  voiceMembersByChannel,
  unreadFor,
  visibleChannelsFor,
  onCreateCommunity,
  onJoinCommunity,
  onPick,
  renderChannel,
}) => {
  const { t } = useTranslation(["sidebar", "common"]);
  return (
    <div className="sb-view">
      {!isFiltering && (
        <>
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
        </>
      )}
      <div className="sb-vhead">
        <span className="sb-vhead-label">{t("communities")}</span>
      </div>
      {communities.length === 0 && (
        <div className="sb-empty">
          {isFiltering ? t("noMatches") : t("noCommunities")}
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
        const channels = isFiltering ? visibleChannelsFor(community.id) : [];
        return (
          <div key={community.id}>
            <div
              role="button"
              tabIndex={0}
              className="sb-srv"
              onClick={() => onPick(community)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onPick(community);
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
              {communityUnread > 0 && (
                <span className="sb-community-unread">{communityUnread}</span>
              )}
              <Icon name="chev" size={13} className="sb-srv-chev collapsed" />
            </div>
            {isFiltering && channels.length > 0 && (
              <div className="sb-srv-channels">
                {channels.map((c) => renderChannel(community, c))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};

export default CommunityList;
