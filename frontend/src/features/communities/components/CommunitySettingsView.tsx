import React from "react";
import { useTranslation } from "react-i18next";
import Icon, { type IconName } from "../../../shared/ui/Icon";
import { communityAPI } from "../api/community";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { usePresenceStore } from "../../social/store/presenceStore";
import { ApiError } from "../../../shared/lib/apiError";
import { colorForId } from "../../../shared/lib/avatar";
import { CommunityLogo } from "../../../shared/ui/Avatar";
import { useUserNamesStore } from "../../social/store/userNamesStore";
import { PERM_MANAGE_CHANNELS, PERM_MANAGE_INVITES, PERM_KICK_MEMBERS, PERM_MANAGE_COMMUNITY, topRoleFor, useRolesStore, userPermissions } from "../store/rolesStore";
import type { Friendship } from "../../../shared/types/friendship";
import DangerSection from "./communitySettings/DangerSection";
import RolesSection from "./communitySettings/RolesSection";
import InvitesSection from "./communitySettings/InvitesSection";
import MembersSection from "./communitySettings/MembersSection";
import ChannelsSection from "./communitySettings/ChannelsSection";
import OverviewSection from "./communitySettings/OverviewSection";
import type { Community } from "../../../shared/types/community";
import type { Channel, ChannelKind, ChannelVisibility } from "../../../shared/types/channel";

type SectionId = "overview" | "channels" | "roles" | "invites" | "members" | "danger";

const SECTIONS: { id: SectionId; icon: IconName }[] = [
  { id: "overview", icon: "grid" },
  { id: "channels", icon: "hash" },
  { id: "roles", icon: "shield" },
  { id: "invites", icon: "user-plus" },
  { id: "members", icon: "users" },
];

interface CommunitySettingsViewProps {
  community: Community;
  friends: Friendship[];
  channels: Channel[];
  channelsState?: "loading" | "error" | "ready";
  onRetryChannels?: () => void;
  currentUserId?: string;
  onClose: () => void;
  onDeleteCommunity: (community: Community) => void;
  onCreateChannel: (
    name: string,
    kind: ChannelKind,
    visibility: ChannelVisibility,
    password?: string,
  ) => Promise<void>;
  onDeleteChannel: (channel: Channel) => void;
}

const CommunitySettingsView: React.FC<CommunitySettingsViewProps> = ({
  community,
  friends,
  channels,
  channelsState,
  onRetryChannels,
  currentUserId,
  onClose,
  onDeleteCommunity,
  onCreateChannel,
  onDeleteChannel,
}) => {
  const { t } = useTranslation(["communitySettings", "common", "modals"]);
  const [section, setSection] = React.useState<SectionId>("overview");
  React.useEffect(() => {
    if (section === "invites") void useFriendshipStore.getState().loadAll();
  }, [section]);
  const isOwner = currentUserId === community.ownerId;

  const rolesData = useRolesStore((st) => st.byCommunity[community.id]);
  const fetchRoles = useRolesStore((st) => st.fetch);
  React.useEffect(() => {
    void fetchRoles(community.id).catch(() => {});
  }, [community.id, fetchRoles]);
  const myPerms = currentUserId
    ? userPermissions(rolesData, currentUserId)
    : 0;
  const myTopPosition = isOwner
    ? Number.POSITIVE_INFINITY
    : currentUserId
      ? (topRoleFor(rolesData, currentUserId)?.position ?? Number.NEGATIVE_INFINITY)
      : Number.NEGATIVE_INFINITY;
  const canManageCommunity = isOwner || (myPerms & PERM_MANAGE_COMMUNITY) !== 0;
  const canManageChannels = isOwner || (myPerms & PERM_MANAGE_CHANNELS) !== 0;
  const canManageInvites = isOwner || (myPerms & PERM_MANAGE_INVITES) !== 0;
  const canKickMembers = isOwner || (myPerms & PERM_KICK_MEMBERS) !== 0;
  const outranksMember = (memberId: string) => {
    if (isOwner) return true;
    if (memberId === community.ownerId) return false;
    const targetTop = topRoleFor(rolesData, memberId)?.position;
    return (
      myTopPosition !== Number.NEGATIVE_INFINITY &&
      (targetTop === undefined || targetTop < myTopPosition)
    );
  };
  const canKick = (memberId: string) =>
    canKickMembers && memberId !== currentUserId && outranksMember(memberId);
  const [kickError, setKickError] = React.useState<string | null>(null);
  const kickMember = async (memberId: string) => {
    setKickError(null);
    try {
      await communityAPI.kickMember(community.id, memberId);
      setMembers((prev) => prev.filter((m) => m.id !== memberId));
      void fetchRoles(community.id).catch(() => {});
    } catch (e) {
      setKickError(e instanceof ApiError ? e.message : t("common:error"));
    }
  };
  const visibleSections = SECTIONS.filter(
    (sec) => sec.id !== "invites" || canManageInvites,
  );
  React.useEffect(() => {
    if (canManageInvites) return;
    setSection((cur) => (cur === "invites" ? "overview" : cur));
  }, [canManageInvites]);

  const [members, setMembers] = React.useState<{ id: string; name: string }[]>([]);
  const [membersTick, setMembersTick] = React.useState(0);
  React.useEffect(() => {
    const onChanged = (e: Event) => {
      const detail = (e as CustomEvent<{ communityId?: string }>).detail;
      if (detail?.communityId === community.id) setMembersTick((n) => n + 1);
    };
    window.addEventListener("rift:community-members-changed", onChanged);
    return () =>
      window.removeEventListener("rift:community-members-changed", onChanged);
  }, [community.id]);
  const nameSyncTick = useUserNamesStore((s) => s.syncTick);
  const [invitedIds, setInvitedIds] = React.useState<Set<string>>(new Set());

  const online = usePresenceStore((s) => s.online);
  const selfOnline = usePresenceStore((s) => s.selfOnline);

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [allMembers, invitesRes] = await Promise.all([
          communityAPI.listAllMembers(community.id),
          communityAPI.listSentInvites(community.id),
        ]);
        if (cancelled) return;
        setMembers(allMembers);
        setInvitedIds(new Set(invitesRes.recipientIds));
      } catch {
        return;
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [community.id, nameSyncTick, membersTick]);

  const memberIds = React.useMemo(
    () => new Set(members.map((m) => m.id)),
    [members],
  );


  const accent = community.accentColor ?? colorForId(community.id);

  const onlineCount = members.filter((m) =>
    m.id === currentUserId ? selfOnline : !!online[m.id],
  ).length;



  return (
    <div className="settings-overlay">
      <nav className="settings-rail">
        <div className="ssv-identity">
          <CommunityLogo
            communityId={community.id}
            name={community.name}
            className="ssv-identity-ava"
            style={{ background: accent }}
          />
          <div className="ssv-identity-info">
            <div className="ssv-identity-name">{community.name}</div>
            <div className="ssv-identity-sub">
              {t("memberCount", { count: members.length })} ·{" "}
              {t("onlineCount", { count: onlineCount })}
            </div>
          </div>
        </div>
        <div className="rail-section">{t("navTitle")}</div>
        {visibleSections.map((s) => (
          <button
            key={s.id}
            type="button"
            className={"rail-item" + (section === s.id ? " active" : "")}
            onClick={() => setSection(s.id)}
          >
            <span className="ic">
              <Icon name={s.icon} size={15} />
            </span>
            {t(`nav.${s.id}`)}
          </button>
        ))}
        <div className="ssv-rail-divider" />
        <button
          type="button"
          className={"rail-item danger" + (section === "danger" ? " active" : "")}
          onClick={() => setSection("danger")}
        >
          <span className="ic">
            <Icon name={isOwner ? "trash" : "logout"} size={15} />
          </span>
          {t("nav.danger")}
        </button>
        <div className="ssv-rail-footer">
          <button type="button" className="ssv-back-btn" onClick={onClose}>
            <Icon name="chev-left" size={14} />
            {t("backToCommunity")}
          </button>
          <div id="community-settings-voice-dock" className="settings-voice-dock" />
        </div>
      </nav>

      <div className="settings-pane">
        <button className="settings-close" onClick={onClose} title="Close (Esc)">
          <Icon name="x" size={16} />
          <span className="esc">ESC</span>
        </button>
        <div className="settings-content">
          {section === "overview" && (
            <OverviewSection
              community={community}
              canManageCommunity={canManageCommunity}
            />
          )}

          {section === "channels" && (
            <ChannelsSection
              community={community}
              channels={channels}
              channelsState={channelsState}
              onRetryChannels={onRetryChannels}
              canManageChannels={canManageChannels}
              onCreateChannel={onCreateChannel}
              onDeleteChannel={onDeleteChannel}
            />
          )}

          {section === "roles" && (
            <RolesSection
              community={community}
              members={members}
              isOwner={isOwner}
              myPerms={myPerms}
              myTopPosition={myTopPosition}
            />
          )}

          {section === "invites" && canManageInvites && (
            <InvitesSection
              community={community}
              friends={friends}
              memberIds={memberIds}
              invitedIds={invitedIds}
              setInvitedIds={setInvitedIds}
              onAlreadyMember={(memberId, name) =>
                setMembers((prev) =>
                  prev.some((m) => m.id === memberId)
                    ? prev
                    : [...prev, { id: memberId, name }],
                )
              }
              canManageInvites={canManageInvites}
            />
          )}

          {section === "members" && (
            <MembersSection
              community={community}
              members={members}
              currentUserId={currentUserId}
              rolesData={rolesData}
              canKick={canKick}
              kickMember={kickMember}
              kickError={kickError}
            />
          )}

          {section === "danger" && (
            <DangerSection
              community={community}
              isOwner={isOwner}
              onClose={onClose}
              onDeleteCommunity={onDeleteCommunity}
            />
          )}
        </div>
      </div>
    </div>
  );
};

export default CommunitySettingsView;
