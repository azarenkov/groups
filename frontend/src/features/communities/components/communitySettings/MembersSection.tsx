import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import ConfirmModal from "../../../../shared/ui/ConfirmModal";
import { usePresenceStore } from "../../../social/store/presenceStore";
import { useUserNamesStore } from "../../../social/store/userNamesStore";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import { topRoleFor, useRolesStore } from "../../store/rolesStore";
import type { Community } from "../../../../shared/types/community";


interface MembersSectionProps {
  community: Community;
  members: { id: string; name: string }[];
  currentUserId?: string;
  rolesData: ReturnType<typeof useRolesStore.getState>["byCommunity"][string] | undefined;
  canKick: (memberId: string) => boolean;
  kickBlocked: (memberId: string) => boolean;
  kickMember: (memberId: string) => Promise<void>;
  kickError: string | null;
}

export const MembersSection: React.FC<MembersSectionProps> = ({
  community,
  members,
  currentUserId,
  rolesData,
  canKick,
  kickBlocked,
  kickMember,
  kickError,
}) => {
  const { t } = useTranslation(["communitySettings", "common", "modals"]);
  const [memberQuery, setMemberQuery] = React.useState("");
  const nameOverrides = useUserNamesStore((st) => st.overrides);
  const effectiveMembers = React.useMemo(
    () =>
      members.map((m) =>
        nameOverrides[m.id] ? { ...m, name: nameOverrides[m.id] } : m,
      ),
    [members, nameOverrides],
  );
  const memberQ = memberQuery.trim().toLowerCase();
  const visibleMembers = memberQ
    ? effectiveMembers.filter((m) => m.name.toLowerCase().includes(memberQ))
    : effectiveMembers;
  const online = usePresenceStore((s) => s.online);
  const selfOnline = usePresenceStore((s) => s.selfOnline);
  const [kickTarget, setKickTarget] = React.useState<{ id: string; name: string } | null>(null);

  return (
            <>
              <h1>{t("nav.members")}</h1>
      {kickError && <div className="ssv-error">{kickError}</div>}
              <p className="lead">
                {t("membersLead", { count: members.length, community: community.name })}
              </p>
              <div className="ssv-search">
                <Icon name="search" size={15} />
                <input
                  placeholder={t("searchMembers")}
                  value={memberQuery}
                  onChange={(e) => setMemberQuery(e.target.value)}
                />
              </div>
              <div className="ssv-card">
                {visibleMembers.length === 0 && (
                  <div className="ssv-empty">{t("noMembers")}</div>
                )}
                {visibleMembers.map((m, i) => {
                  const isOnline =
                    m.id === currentUserId ? selfOnline : !!online[m.id];
                  return (
                    <React.Fragment key={m.id}>
                      {i > 0 && <div className="ssv-divider" />}
                      <div className={"ssv-row" + (isOnline ? "" : " off")}>
                        <div className="ssv-row-ava-wrap">
                          <UserAvatar
                            userId={m.id}
                            name={m.name}
                            className="ssv-row-ava"
                          />
                          {isOnline && <span className="ssv-row-dot" />}
                        </div>
                        <div className="ssv-row-main">
                          <div className="ssv-row-name">{m.name}</div>
                          <div className="ssv-row-sub">
                            {isOnline
                              ? t("common:statusOnline", {
                                  defaultValue: t("online"),
                                })
                              : t("offline")}
                          </div>
                        </div>
                        {(() => {
                          const top = topRoleFor(rolesData, m.id);
                          return top ? (
                            <span
                              className="ssv-tag"
                              style={{
                                color: top.color,
                                background: `color-mix(in srgb, ${top.color} 16%, transparent)`,
                              }}
                            >
                              {top.name}
                            </span>
                          ) : null;
                        })()}
                        <span
                          className={
                            "ssv-tag" + (m.id === community.ownerId ? " owner" : "")
                          }
                        >
                          {m.id === community.ownerId ? t("roleOwner") : t("roleMember")}
                        </span>
                        {canKick(m.id) ? (
                          <button
                            type="button"
                            className="ssv-row-delete"
                            title={t("kickMember")}
                            aria-label={t("kickMember")}
                            onClick={() => setKickTarget({ id: m.id, name: m.name })}
                          >
                            <Icon name="logout" size={14} />
                          </button>
                        ) : kickBlocked(m.id) ? (
                          <span className="ssv-kick-blocked" title={t("kickBlockedRank")} aria-label={t("kickBlockedRank")}>
                            <Icon name="logout" size={14} />
                          </span>
                        ) : null}
                      </div>
                    </React.Fragment>
                  );
                })}
              </div>
      {kickTarget && (
        <ConfirmModal
          title={t("kickConfirmTitle", { name: kickTarget.name })}
          subtitle={t("kickConfirmSub", { community: community.name })}
          confirmLabel={t("kickConfirm")}
          danger
          onCancel={() => setKickTarget(null)}
          onConfirm={() => {
            const target = kickTarget;
            setKickTarget(null);
            void kickMember(target.id);
          }}
        />
      )}
            </>
  );
};

export default MembersSection;
