import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { communityAPI } from "../../api/community";
import { ApiError } from "../../../../shared/lib/apiError";
import { UserAvatar } from "../../../../shared/ui/Avatar";
import { useUserNamesStore } from "../../../social/store/userNamesStore";
import { topRoleFor, useRolesStore } from "../../store/rolesStore";
import {
  PERMISSIONS,
  PERMISSION_LABEL_KEYS,
  isSubset,
  union,
  without,
} from "../../lib/permissions";
import type { Permission } from "../../lib/permissions";
import type { Community } from "../../../../shared/types/community";


interface RolesSectionProps {
  community: Community;
  members: { id: string; name: string }[];
  isOwner: boolean;
  myPerms: Permission[];
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
  const [newRoleName, setNewRoleName] = React.useState("");
  const [newRoleColor, setNewRoleColor] = React.useState(ROLE_COLORS[0]);
  const [newRolePerms, setNewRolePerms] = React.useState<Permission[]>([]);
  const [roleBusy, setRoleBusy] = React.useState(false);
  const [roleError, setRoleError] = React.useState<string | null>(null);
  const [expandedRoleId, setExpandedRoleId] = React.useState<string | null>(null);
  const [rolePermsDraft, setRolePermsDraft] = React.useState<Record<string, Permission[]>>({});

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
      setNewRolePerms([]);
      refreshRoles();
    } catch (e) {
      setRoleError(e instanceof ApiError ? e.message : t("common:error"));
    } finally {
      setRoleBusy(false);
    }
  };


  type RoleDraft = { name?: string; color?: string; add: Permission[]; remove: Permission[] };
  const roleWriteChainsRef = React.useRef<Record<string, Promise<void>>>({});
  const roleLatestRef = React.useRef<Record<string, RoleDraft>>({});

  const enqueueRoleEdit = (
    role: { id: string; name: string; color: string; permissions: Permission[] },
    patch: { name?: string; color?: string; add?: Permission[]; remove?: Permission[] },
  ) => {
    const base = roleLatestRef.current[role.id] ?? { add: [], remove: [] };
    const next: RoleDraft = {
      name: patch.name ?? base.name,
      color: patch.color ?? base.color,
      add: without(union(base.add, patch.add ?? []), patch.remove ?? []),
      remove: without(union(base.remove, patch.remove ?? []), patch.add ?? []),
    };
    roleLatestRef.current[role.id] = next;
    setRolePermsDraft((prev) => ({
      ...prev,
      [role.id]: without(union(role.permissions, next.add), next.remove),
    }));
    const chain = roleWriteChainsRef.current[role.id] ?? Promise.resolve();
    roleWriteChainsRef.current[role.id] = chain.then(async () => {
      const target = roleLatestRef.current[role.id];
      if (!target) return;
      const empty =
        target.name === undefined &&
        target.color === undefined &&
        target.add.length === 0 &&
        target.remove.length === 0;
      if (empty) {
        delete roleLatestRef.current[role.id];
        return;
      }
      delete roleLatestRef.current[role.id];
      try {
        await communityAPI.updateRole(community.id, role.id, {
          name: target.name,
          color: target.color,
          add: target.add,
          remove: target.remove,
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
    role: { id: string; name: string; color: string; permissions: Permission[] },
    permission: Permission,
    checked: boolean,
  ) => {
    enqueueRoleEdit(role, checked ? { add: [permission] } : { remove: [permission] });
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
  const canManageRoles = isOwner || myPerms.includes("manageRoles");
  const canTouchRole = (position: number) => canManageRoles && position < myTopPosition;
  const grantable: readonly Permission[] = isOwner ? PERMISSIONS : myPerms;
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
                    {PERMISSIONS.map((perm) => {
                      const on = newRolePerms.includes(perm);
                      const locked = roleBusy || (!on && !grantable.includes(perm));
                      return (
                      <div key={perm} className="ssv-role-perm">
                        <span>{t(PERMISSION_LABEL_KEYS[perm])}</span>
                        <button
                          type="button"
                          className={"s-switch" + (on ? " on" : "")}
                          disabled={locked}
                          onClick={() =>
                            setNewRolePerms((prev) =>
                              on ? without(prev, [perm]) : union(prev, [perm]),
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
                        className={"ssv-row ssv-role-row" + (expanded ? " expanded" : "")}
                        role="button"
                        tabIndex={0}
                        aria-expanded={canTouchRole(role.position) ? expanded : undefined}
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
                            {role.permissions
                              .map((p2) => t(PERMISSION_LABEL_KEYS[p2]))
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
                              <Icon name="arrow-up" size={14} />
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
                              <Icon name="arrow-down" size={14} />
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
                        {canTouchRole(role.position) && (
                          <span className="ssv-role-expand" aria-hidden="true">
                            <Icon name="chev" size={18} />
                          </span>
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
                            {PERMISSIONS.map((perm) => {
                              const on = (rolePermsDraft[role.id] ?? role.permissions).includes(perm);
                              const locked = on
                                ? !outranksAllHolders(role.id)
                                : !grantable.includes(perm);
                              return (
                                <div key={perm} className="ssv-role-perm">
                                  <span>{t(PERMISSION_LABEL_KEYS[perm])}</span>
                                  <button
                                    type="button"
                                    className={"s-switch" + (on ? " on" : "")}
                                    disabled={locked}
                                    onClick={() =>
                                      toggleRolePerm(role, perm, !on)
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
                                        !isSubset(role.permissions, grantable))
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
