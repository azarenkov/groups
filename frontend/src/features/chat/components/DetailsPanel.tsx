import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../shared/ui/Icon";
import { UserAvatar } from "../../../shared/ui/Avatar";
import { colorForId } from "../../../shared/lib/avatar";
import { useAvatarUrl } from "../../../shared/lib/avatarCache";
import ProBadge from "../../subscription/components/ProBadge";
import EmojiStatusIcon from "../../subscription/components/EmojiStatusIcon";
import { useBadge } from "../../subscription/hooks/useBadge";
import { useActiveEmojiStatus } from "../../subscription/lib/emojiStatus";
import ProfileFacts from "../../social/components/ProfileFacts";
import { useUserProfileDetails } from "../../social/hooks/useUserProfileDetails";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import MembersPanel from "../../communities/components/MembersPanel";
import SharedPanel from "./SharedPanel";
import NotificationLevelSection from "./NotificationLevelSection";
import { useDetailsPanelStore } from "../store/detailsPanelStore";
import type { ChannelDetailsTab, DmDetailsTab } from "../store/detailsPanelStore";

type DetailsTarget =
  | { kind: "dm"; friendId: string; name: string; color?: string; isFriend?: boolean }
  | { kind: "channel"; communityId: string; channelId: string; name: string };

interface DetailsPanelProps {
  target: DetailsTarget;
  currentUserId?: string;
  onShowMessage: (messageId: string) => void;
}

const DmProfile: React.FC<{ friendId: string; name: string; color?: string }> = ({ friendId, name, color }) => {
  const { t } = useTranslation("profile");
  const { profile, failed, connections, blockedMe, bio } = useUserProfileDetails(friendId);
  const badge = useBadge(friendId);
  const emojiStatus = useActiveEmojiStatus(badge?.emojiStatus);
  const bannerUrl = useAvatarUrl("banner", badge && !blockedMe ? friendId : null);
  const shownName = profile?.name ?? name;
  return (
    <div className="details-profile">
      <div
        className={"details-banner" + (bannerUrl ? " image" : "")}
        style={
          bannerUrl
            ? { backgroundImage: `url(${bannerUrl})` }
            : { background: `color-mix(in srgb, ${color ?? colorForId(friendId)} 28%, var(--surface-raised))` }
        }
      />
      <button
        type="button"
        className="details-avatar"
        title={t("viewProfile")}
        aria-label={t("viewProfile")}
        onClick={() => useProfileModalStore.getState().open(friendId)}
      >
        <UserAvatar userId={friendId} name={shownName} className="details-avatar-img" style={{ background: color ?? colorForId(friendId) }} />
      </button>
      <div className="details-name-row">
        <span className="details-name">{shownName}</span>
        {badge && !blockedMe &&
          (emojiStatus ? (
            <EmojiStatusIcon status={emojiStatus} size="md" />
          ) : (
            <ProBadge tier={badge.tier} badgeStyle={badge.badgeStyle} memberNumber={badge.memberNumber} size="sm" />
          ))}
      </div>
      <div className="details-handle">@{shownName}</div>
      {failed ? (
        <div className="details-muted">{t("unavailable")}</div>
      ) : (
        <div className="details-facts">
          <ProfileFacts profile={profile} connections={connections} bio={bio} blockedMe={blockedMe} />
        </div>
      )}
      <button type="button" className="details-link" onClick={() => useProfileModalStore.getState().open(friendId)}>
        {t("viewProfile")}
      </button>
    </div>
  );
};

const DetailsPanel: React.FC<DetailsPanelProps> = ({ target, currentUserId, onShowMessage }) => {
  const { t } = useTranslation("chat");
  const dmTab = useDetailsPanelStore((s) => s.dmTab);
  const channelTab = useDetailsPanelStore((s) => s.channelTab);
  const close = () => useDetailsPanelStore.getState().setOpen(false);

  const tabs: { id: DmDetailsTab | ChannelDetailsTab; label: string }[] =
    target.kind === "dm"
      ? [
          { id: "profile", label: t("details.profile") },
          { id: "shared", label: t("details.shared") },
        ]
      : [
          { id: "channel", label: t("details.channel") },
          { id: "members", label: t("details.members") },
          { id: "shared", label: t("details.shared") },
        ];
  const active = target.kind === "dm" ? dmTab : channelTab;
  const pick = (id: string) =>
    target.kind === "dm"
      ? useDetailsPanelStore.getState().showDm(id as DmDetailsTab)
      : useDetailsPanelStore.getState().showChannel(id as ChannelDetailsTab);

  return (
    <aside className="details-panel" aria-label={t("details.title")}>
      <div className="details-head">
        <div className="details-tabs" role="tablist">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={active === tab.id}
              className={"details-tab" + (active === tab.id ? " on" : "")}
              onClick={() => pick(tab.id)}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <button type="button" className="icon-btn details-close" title={t("details.close")} aria-label={t("details.close")} onClick={close}>
          <Icon name="x" size={15} />
        </button>
      </div>
      <div className="details-body">
        {target.kind === "dm" && active === "profile" && (
          <>
            <DmProfile friendId={target.friendId} name={target.name} color={target.color} />
            {target.isFriend !== false && <NotificationLevelSection dmId={target.friendId} />}
          </>
        )}
        {target.kind === "channel" && active === "channel" && (
          <>
            <section className="details-section">
              <div className="details-channel-name">
                <Icon name="hash" size={18} />
                <span>{target.name}</span>
              </div>
            </section>
            <NotificationLevelSection channelId={target.channelId} />
          </>
        )}
        {target.kind === "channel" && active === "members" && (
          <MembersPanel key={target.channelId} communityId={target.communityId} channelId={target.channelId} />
        )}
        {active === "shared" && (
          <SharedPanel
            key={target.kind === "dm" ? `dm:${target.friendId}` : `ch:${target.channelId}`}
            scope={
              target.kind === "dm"
                ? { kind: "dm", friendId: target.friendId }
                : { kind: "channel", communityId: target.communityId, channelId: target.channelId }
            }
            currentUserId={currentUserId}
            onShowMessage={onShowMessage}
          />
        )}
      </div>
    </aside>
  );
};

export default DetailsPanel;
