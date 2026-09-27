import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { communityAPI } from "../../api/community";
import { ApiError } from "../../../../shared/lib/apiError";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import { useUserNamesStore } from "../../../social/store/userNamesStore";
import { PERM_MANAGE_CHANNELS, PERM_MANAGE_INVITES, PERM_MANAGE_MESSAGES, PERM_MANAGE_ROLES, PERM_KICK_MEMBERS, PERM_MANAGE_COMMUNITY, PERM_DEAFEN_MEMBERS, PERM_DISCONNECT_MEMBERS, PERM_MUTE_MEMBERS, topRoleFor, useRolesStore } from "../../store/rolesStore";
import type { Community } from "../../../../shared/types/community";


interface RolesSectionProps {
  community: Community;
  members: { id: string; name: string }[];
  isOwner: boolean;
  myPerms: number;
  myTopPosition: number;
}

export const RolesSection: React.FC<RolesSectionProps> = ({
  community,
  members,
  isOwner,
  myPerms,
  myTopPosition,
}) => {
  const { t } = useTranslation(["communitySettings", "common", "modals"]);
  const ROLE_COLORS = [
    "#5865f2", "#3ba55c", "#faa61a", "#ed4245", "#eb459e", "#9b59b6", "#00b0f4", "#8e8e93",
  ];
  const PERMS: { bit: number; key: string }[] = [
    { bit: PERM_MANAGE_CHANNELS, key: "permManageChannels" },
    { bit: PERM_MANAGE_INVITES, key: "permManageInvites" },
    { bit: PERM_MANAGE_MESSAGES, key: "permManageMessages" },
    { bit: PERM_MUTE_MEMBERS, key: "permMuteMembers" },
    { bit: PERM_DEAFEN_MEMBERS, key: "permDeafenMembers" },
    { bit: PERM_DISCONNECT_MEMBERS, key: "permDisconnectMembers" },
    { bit: PERM_MANAGE_ROLES, key: "permManageRoles" },
    { bit: PERM_MANAGE_COMMUNITY, key: "permManageCommunity" },
    { bit: PERM_KICK_MEMBERS, key: "permKickMembers" },
  ];
  const [newRoleName, setNewRoleName] = React.useState("");
  const [newRoleColor, setNewRoleColor] = React.useState(ROLE_COLORS[0]);
  const [newRolePerms, setNewRolePerms] = React.useState(0);
  const [roleBusy, setRoleBusy] = React.useState(false);
  const [roleError, setRoleError] = React.useState<string | null>(null);
  const [expandedRoleId, setExpandedRoleId] = React.useState<string | null>(null);
  const [rolePermsDraft, setRolePermsDraft] = React.useState<Record<string, number>>({});

  const fetchRoles = useRolesStore((st) => st.fetch);
  const refreshRoles = () => void fetchRoles(community.id).catch(() => {});

  const submitCreateRole = async () => {
    const name = newRoleName.trim();
    if (!name) {
      setRoleError(t("roleNameRequired"));
      return;
    }
    setRoleBusy(true);
    setRoleError(null);
    try {
      await communityAPI.createRole(community.id, name, newRoleColor, newRolePerms);
      setNewRoleName("");
      setNewRolePerms(0);
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    } finally {
      setRoleBusy(false);
    }
  };


  type RoleDraft = { name?: string; color?: string; add: number; remove: number };
  const roleWriteChainsRef = React.useRef<Record<string, Promise<void>>>({});
  const roleLatestRef = React.useRef<Record<string, RoleDraft>>({});

  const enqueueRoleEdit = (
    role: { id: string; name: string; color: string; permissions: number },
    patch: { name?: string; color?: string; add?: number; remove?: number },
  ) => {
    const base = roleLatestRef.current[role.id] ?? { add: 0, remove: 0 };
    const next: RoleDraft = {
      name: patch.name ?? base.name,
      color: patch.color ?? base.color,
      add: (base.add | (patch.add ?? 0)) & ~(patch.remove ?? 0),
      remove: (base.remove | (patch.remove ?? 0)) & ~(patch.add ?? 0),
    };
    roleLatestRef.current[role.id] = next;
    setRolePermsDraft((prev) => ({
      ...prev,
      [role.id]: (role.permissions | next.add) & ~next.remove,
    }));
    const chain = roleWriteChainsRef.current[role.id] ?? Promise.resolve();
    roleWriteChainsRef.current[role.id] = chain.then(async () => {
      const target = roleLatestRef.current[role.id];
      if (!target) return;
      const empty =
        target.name === undefined &&
        target.color === undefined &&
        target.add === 0 &&
        target.remove === 0;
      if (empty) {
        delete roleLatestRef.current[role.id];
        return;
      }
      delete roleLatestRef.current[role.id];
      try {
        await communityAPI.updateRole(community.id, role.id, {
          name: target.name,
          color: target.color,
          permissionsAdd: target.add || undefined,
          permissionsRemove: target.remove || undefined,
        });
        if (!roleLatestRef.current[role.id]) {
          setRolePermsDraft((cur) => {
            const nextDraft = { ...cur };
            delete nextDraft[role.id];
            return nextDraft;
          });
        }
        refreshRoles();
      } catch (e) {
        setRoleError(e instanceof ApiError ? e.message : t("common:error"));
        if (!roleLatestRef.current[role.id]) {
          setRolePermsDraft((cur) => {
            const nextDraft = { ...cur };
            delete nextDraft[role.id];
            return nextDraft;
          });
        }
        refreshRoles();
      }
    });
  };

  const toggleRolePerm = (
    role: { id: string; name: string; color: string; permissions: number },
    bit: number,
    checked: boolean,
  ) => {
    enqueueRoleEdit(role, checked ? { add: bit } : { remove: bit });
  };

  const swapRoles = async (
    a: { id: string; position: number },
    b: { id: string; position: number },
  ) => {
    try {
      await communityAPI.swapRoles(community.id, a.id, b.id);
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    }
  };

  const removeRole = async (roleId: string) => {
    try {
      await communityAPI.deleteRole(community.id, roleId);
      if (expandedRoleId === roleId) setExpandedRoleId(null);
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    }
  };

  const toggleAssignment = async (roleId: string, memberId: string, has: boolean) => {
    try {
      if (has) await communityAPI.revokeRole(community.id, memberId, roleId);
      else await communityAPI.assignRole(community.id, memberId, roleId);
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    }
  };

  const rolesData = useRolesStore((st) => st.byCommunity[community.id]);
  const canManageRoles = isOwner || (myPerms & PERM_MANAGE_ROLES) !== 0;
  const canTouchRole = (position: number) => canManageRoles && position < myTopPosition;
  const grantableBits = isOwner ? ~0 : myPerms;
  const outranksMember = (memberId: string) => {
    if (isOwner) return true;
    if (memberId === community.ownerId) return false;
    const targetTop = topRoleFor(rolesData, memberId)?.position;
    return (
      myTopPosition !== Number.NEGATIVE_INFINITY &&
      (targetTop === undefined || targetTop < myTopPosition)
    );
  };
  const outranksAllHolders = (roleId: string) => {
    if (isOwner) return true;
    const holders = Object.entries(rolesData?.memberRoles ?? {})
      .filter(([, ids]) => ids.includes(roleId))
      .map(([memberId]) => memberId);
    return holders.every((memberId) => outranksMember(memberId));
  };
  const nameOverrides = useUserNamesStore((st) => st.overrides);
  const effectiveMembers = React.useMemo(
    () =>
      members.map((m) =>
        nameOverrides[m.id] ? { ...m, name: nameOverrides[m.id] } : m,
      ),
    [members, nameOverrides],
  );

  return (
            <>
              <h1>{t("nav.roles")}</h1>
              <p className="lead">{t("rolesLead", { community: community.name })}</p>
              {canManageRoles && (
                <form
                  className="ssv-invite-card"
                  onSubmit={(e) => {
                    e.preventDefault();
                    void submitCreateRole();
                  }}
                >
                  <div className="ssv-invite-head">
                    <Icon name="shield" size={16} />
                    <span>{t("createRole")}</span>
                  </div>
                  <div className="ssv-invite-row">
                    <input
                      className="ssv-input ssv-create-input"
                      placeholder={t("roleName")}
                      value={newRoleName}
                      maxLength={40}
                      disabled={roleBusy}
                      onChange={(e) => {
                        setNewRoleName(e.target.value);
                        setRoleError(null);
                      }}
                    />
                    <button type="submit" className="ssv-btn primary" disabled={roleBusy}>
                      <Icon name="plus" size={14} />
                      {t("createRole")}
                    </button>
                  </div>
                  <div className="ssv-role-colors">
                    {ROLE_COLORS.map((c) => (
                      <button
                        key={c}
                        type="button"
                        className={
                          "ssv-role-color" + (newRoleColor === c ? " on" : "")
                        }
                        style={{ background: c }}
                        onClick={() => setNewRoleColor(c)}
                      />
                    ))}
                  </div>
                  <div className="ssv-role-perms">
                    {PERMS.map((perm) => {
                      const on = (newRolePerms & perm.bit) !== 0;
                      const locked =
                        roleBusy || (!on && (grantableBits & perm.bit) === 0);
                      return (
                      <div key={perm.bit} className="ssv-role-perm">
                        <span>{t(perm.key)}</span>
                        <button
                          type="button"
                          className={"s-switch" + (on ? " on" : "")}
                          disabled={locked}
                          onClick={() =>
                            setNewRolePerms((prev) =>
                              on ? prev & ~perm.bit : prev | perm.bit,
                            )
                          }
                        />
                      </div>
                      );
                    })}
                  </div>
                </form>
              )}
              {roleError && <div className="ssv-error">{roleError}</div>}
              <div className="ssv-section-label">
                {t("roleList", { count: rolesData?.roles.length ?? 0 })}
              </div>
              <div className="ssv-card">
                {(rolesData?.roles.length ?? 0) === 0 && (
                  <div className="ssv-empty">{t("noRoles")}</div>
                )}
                {(() => {
                  const sortedRoles = rolesData?.roles ?? [];
                  return sortedRoles.map((role, i) => {
                  const expanded = expandedRoleId === role.id;
                  const holders = effectiveMembers.filter((m) =>
                    (rolesData?.memberRoles[m.id] ?? []).includes(role.id),
                  );
                  return (
                    <React.Fragment key={role.id}>
                      {i > 0 && <div className="ssv-divider" />}
                      <div
                        className="ssv-row ssv-role-row"
                        role="button"
                        tabIndex={0}
                        onClick={() =>
                          setExpandedRoleId(expanded ? null : role.id)
                        }
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            setExpandedRoleId(expanded ? null : role.id);
                          }
                        }}
                      >
                        <span
                          className="ssv-role-dot"
                          style={{ background: role.color }}
                        />
                        <div className="ssv-row-main">
                          <div className="ssv-row-name">{role.name}</div>
                          <div className="ssv-row-sub">
                            {PERMS.filter((p2) => (role.permissions & p2.bit) !== 0)
                              .map((p2) => t(p2.key))
                              .join(" · ") || "—"}
                          </div>
                        </div>
                        <span className="ssv-tag">{holders.length}</span>
                        {canManageRoles && (
                          <span
                            className="ssv-role-reorder"
                            onClick={(e) => e.stopPropagation()}
                          >
                            <button
                              type="button"
                              className="ssv-row-delete"
                              disabled={
                                i === 0 ||
                                !canTouchRole(role.position) ||
                                !canTouchRole(sortedRoles[i - 1].position)
                              }
                              title="↑"
                              onClick={() => void swapRoles(role, sortedRoles[i - 1])}
                            >
                              <Icon name="chev" size={13} className="flip-y" />
                            </button>
                            <button
                              type="button"
                              className="ssv-row-delete"
                              disabled={
                                i === sortedRoles.length - 1 ||
                                !canTouchRole(role.position) ||
                                !canTouchRole(sortedRoles[i + 1].position)
                              }
                              title="↓"
                              onClick={() => void swapRoles(role, sortedRoles[i + 1])}
                            >
                              <Icon name="chev" size={13} />
                            </button>
                          </span>
                        )}
                        {canTouchRole(role.position) && outranksAllHolders(role.id) && (
                          <button
                            type="button"
                            className="ssv-row-delete"
                            title={t("deleteRole")}
                            onClick={(e) => {
                              e.stopPropagation();
                              void removeRole(role.id);
                            }}
                          >
                            <Icon name="trash" size={14} />
                          </button>
                        )}
                      </div>
                      {expanded && canTouchRole(role.position) && (
                        <div className="ssv-role-detail">
                          <div className="ssv-role-colors">
                            {ROLE_COLORS.map((c) => (
                              <button
                                key={c}
                                type="button"
                                className={
                                  "ssv-role-color" + (role.color === c ? " on" : "")
                                }
                                style={{ background: c }}
                                onClick={() => enqueueRoleEdit(role, { color: c })}
                              />
                            ))}
                          </div>
                          <div className="ssv-role-perms">
                            {PERMS.map((perm) => {
                              const on =
                                ((rolePermsDraft[role.id] ?? role.permissions) &
                                  perm.bit) !==
                                0;
                              const locked = on
                                ? !outranksAllHolders(role.id)
                                : (grantableBits & perm.bit) === 0;
                              return (
                                <div key={perm.bit} className="ssv-role-perm">
                                  <span>{t(perm.key)}</span>
                                  <button
                                    type="button"
                                    className={"s-switch" + (on ? " on" : "")}
                                    disabled={locked}
                                    onClick={() =>
                                      toggleRolePerm(role, perm.bit, !on)
                                    }
                                  />
                                </div>
                              );
                            })}
                          </div>
                          <div className="ssv-section-label">{t("roleMembers")}</div>
                          <div className="ssv-role-members">
                            {effectiveMembers.map((m) => {
                              const has = (rolesData?.memberRoles[m.id] ?? []).includes(
                                role.id,
                              );
                              return (
                                <div key={m.id} className="ssv-role-member-row">
                                  <UserAvatar
                                    userId={m.id}
                                    name={m.name}
                                    className="ssv-role-member-ava"
                                  />
                                  <span className="ssv-role-member-name">
                                    {m.name}
                                  </span>
                                  <button
                                    type="button"
                                    className={"s-switch" + (has ? " on" : "")}
                                    disabled={
                                      !outranksMember(m.id) ||
                                      (!has &&
                                        (role.permissions & ~grantableBits) !== 0)
                                    }
                                    onClick={() =>
                                      void toggleAssignment(role.id, m.id, has)
                                    }
                                  />
                                </div>
                              );
                            })}
                          </div>
                        </div>
                      )}
                    </React.Fragment>
                  );
                  });
                })()}
              </div>
            </>
  );
};

export default RolesSection;
