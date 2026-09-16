import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { usePresenceStore } from "../../../social/store/presenceStore";
import { useUserNamesStore } from "../../../social/store/userNamesStore";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import { topRoleFor, useRolesStore } from "../../store/rolesStore";
import type { Server } from "../../../../shared/types/server";


interface MembersSectionProps {
  server: Server;
  members: { id: string; name: string }[];
  currentUserId?: string;
  rolesData: ReturnType<typeof useRolesStore.getState>["byServer"][string] | undefined;
  canKick: (memberId: string) => boolean;
  kickMember: (memberId: string) => Promise<void>;
  kickError: string | null;
}

export const MembersSection: React.FC<MembersSectionProps> = ({
  server,
  members,
  currentUserId,
  rolesData,
  canKick,
  kickMember,
  kickError,
}) => {
  const { t } = useTranslation(["serverSettings", "common", "modals"]);
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

  return (
            <>
              <h1>{t("nav.members")}</h1>
      {kickError && <div className="ssv-error">{kickError}</div>}
              <p className="lead">
                {t("membersLead", { count: members.length, server: server.name })}
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
                            "ssv-tag" + (m.id === server.ownerId ? " owner" : "")
                          }
                        >
                          {m.id === server.ownerId ? t("roleOwner") : t("roleMember")}
                        </span>
                        {canKick(m.id) && (
                          <button
                            type="button"
                            className="ssv-row-delete"
                            title={t("kickMember")}
                            onClick={() => void kickMember(m.id)}
                          >
                            <Icon name="logout" size={14} />
                          </button>
                        )}
                      </div>
                    </React.Fragment>
                  );
                })}
              </div>
            </>
  );
};

export default MembersSection;
