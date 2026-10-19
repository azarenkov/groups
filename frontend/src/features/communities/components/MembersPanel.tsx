import React, { useEffect, useState } from "react";
import UserEmojiStatus from "../../social/components/UserEmojiStatus";
import { profileVersion, useUserProfileStore } from "../../social/store/userProfileStore";
import { useProfileModalStore } from "../../social/store/profileModalStore";
import { useTranslation } from "react-i18next";
import { channelAPI } from "../api/channel";
import type { ChannelParticipant } from "../../../shared/types/channel";
import { UserAvatar } from "../../../shared/ui/Avatar";
import { usePresenceStore } from "../../social/store/presenceStore";
import { useUserNamesStore } from "../../social/store/userNamesStore";
import { topRoleFor, useRolesStore } from "../store/rolesStore";
import { useCommunityStore } from "../store/communityStore";
import { groupMembers } from "../lib/memberOrder";

interface MembersPanelProps {
  communityId: string;
  channelId: string;
}

const MembersPanel: React.FC<MembersPanelProps> = ({ communityId, channelId }) => {
  const { t } = useTranslation(["members", "common"]);
  const [members, setMembers] = useState<ChannelParticipant[]>([]);
  useEffect(() => {
    const apply = useUserProfileStore.getState().apply;
    for (const m of members) {
      apply(m.id, m.bio ?? "", profileVersion(m.profileUpdatedAt));
    }
  }, [members]);
  const onlineUsers = usePresenceStore((s) => s.online);
  const nameOverrides = useUserNamesStore((s) => s.overrides);
  const syncTick = useUserNamesStore((s) => s.syncTick);
  const selfUserId = usePresenceStore((s) => s.selfUserId);
  const selfOnline = usePresenceStore((s) => s.selfOnline);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const rolesData = useRolesStore((s) => s.byCommunity[communityId]);
  const communityOwnerId = useCommunityStore(
    (s) => s.communities.find((sv) => sv.id === communityId)?.ownerId,
  );
  const fetchRoles = useRolesStore((s) => s.fetch);
  useEffect(() => {
    void fetchRoles(communityId).catch(() => {});
  }, [communityId, fetchRoles]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    const PER_PAGE = 50;
    const fetchAll = async (): Promise<ChannelParticipant[]> => {
      const acc: ChannelParticipant[] = [];
      let page = 1;
      while (!cancelled) {
        const res = await channelAPI.listParticipants(
          communityId,
          channelId,
          page,
          PER_PAGE,
        );
        acc.push(...res.participants);
        const totalPages = res.pagination?.totalPages ?? 1;
        if (page >= totalPages || res.participants.length < PER_PAGE) break;
        page += 1;
      }
      return acc;
    };

    fetchAll()
      .then((all) => {
        if (!cancelled) setMembers(all);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load members");
          setMembers([]);
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [communityId, channelId, syncTick]);

  const groups = groupMembers(
    members.map((m) => ({
      ...m,
      name: nameOverrides[m.id] ?? m.name,
      online: m.id === selfUserId ? selfOnline : !!onlineUsers[m.id],
      owner: m.id === communityOwnerId,
      rolePosition: topRoleFor(rolesData, m.id)?.position ?? null,
    })),
  );

  const renderMember = (m: (typeof groups.online)[number]) => {
    const isSelf = m.id === selfUserId;
    const isOnline = m.online;
    const displayName = m.name;
    const topRole = topRoleFor(rolesData, m.id);
    const statusLabel = isOnline ? t("common:online") : t("common:offline");
    return (
      <div
        key={m.id}
        className="member-row clickable"
        role="button"
        tabIndex={0}
        onClick={() => useProfileModalStore.getState().open(m.id)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            useProfileModalStore.getState().open(m.id);
          }
        }}
      >
        <UserAvatar userId={m.id} name={displayName} className="ava">
          <span
            className={"presence " + (isOnline ? "online" : "offline")}
            title={statusLabel}
            aria-label={statusLabel}
          />
        </UserAvatar>
        <div className="info">
          <div className="name">
            <span className="name-text">{displayName}</span>
            <UserEmojiStatus userId={m.id} size="sm" />
            {isSelf && <span className="member-you">{t("you")}</span>}
          </div>
          <div className="role">
            {m.id === communityOwnerId ? (
              <span className="member-role-badge owner">{t("owner")}</span>
            ) : (
              topRole && (
                <span
                  className="member-role-badge"
                  style={{
                    color: topRole.color,
                    background: `color-mix(in srgb, ${topRole.color} 16%, transparent)`,
                  }}
                >
                  {topRole.name}
                </span>
              )
            )}
            <span className="member-status">{statusLabel}</span>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="members-panel">
      <div className="mp-section">
        {t("title")}
        <span className="count">— {members.length}</span>
      </div>
      {loading && members.length === 0 && (
        <div className="mp-empty">{t("common:loading")}</div>
      )}
      {error && <div className="mp-empty error-text">{error}</div>}
      {!loading && !error && members.length === 0 && (
        <div className="mp-empty">{t("noMembers")}</div>
      )}
      {groups.online.length > 0 && (
        <div className="mp-group">{t("online", { count: groups.online.length })}</div>
      )}
      {groups.online.map(renderMember)}
      {groups.offline.length > 0 && (
        <div className="mp-group">{t("offline", { count: groups.offline.length })}</div>
      )}
      {groups.offline.map(renderMember)}
    </div>
  );
};

export default MembersPanel;
