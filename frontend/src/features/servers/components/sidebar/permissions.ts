import React from "react";
import type { Server } from "../../../../shared/types/server";
import {
  PERM_MANAGE_CHANNELS,
  PERM_MANAGE_INVITES,
  PERM_DEAFEN_MEMBERS,
  PERM_MUTE_MEMBERS,
  topRoleFor,
  useRolesStore,
  userPermissions,
} from "../../store/rolesStore";

export interface SidebarPermissions {
  canInviteIn: (server: Server) => boolean;
  canManageChannelsIn: (server: Server) => boolean;
  outranksIn: (server: Server, memberId: string) => boolean;
  canMuteIn: (server: Server) => boolean;
  canDeafenIn: (server: Server) => boolean;
  canLiftDeafenIn: (
    server: Server,
    m: { id: string; deafenedBy?: string },
  ) => boolean;
}

export const moderatorDeafened = (m: { id: string; deafenedBy?: string }) =>
  !!m.deafenedBy && m.deafenedBy !== m.id;

export function useSidebarPermissions(
  currentUserId: string | undefined,
  openServerId: string | null,
): SidebarPermissions {
  const rolesByServer = useRolesStore((st) => st.byServer);
  const fetchRoles = useRolesStore((st) => st.fetch);
  React.useEffect(() => {
    if (openServerId) void fetchRoles(openServerId).catch(() => {});
  }, [openServerId, fetchRoles]);

  const canInviteIn = (server: Server) =>
    !!currentUserId &&
    (server.ownerId === currentUserId ||
      (userPermissions(rolesByServer[server.id], currentUserId) &
        PERM_MANAGE_INVITES) !==
        0);
  const canManageChannelsIn = (server: Server) =>
    !!currentUserId &&
    (server.ownerId === currentUserId ||
      (userPermissions(rolesByServer[server.id], currentUserId) &
        PERM_MANAGE_CHANNELS) !==
        0);
  const outranksIn = (server: Server, memberId: string) => {
    if (!currentUserId) return false;
    if (server.ownerId === currentUserId) return true;
    if (memberId === server.ownerId) return false;
    const data = rolesByServer[server.id];
    const myTop = topRoleFor(data, currentUserId)?.position;
    if (myTop === undefined) return false;
    const targetTop = topRoleFor(data, memberId)?.position;
    return targetTop === undefined || targetTop < myTop;
  };
  const canMuteIn = (server: Server) =>
    !!currentUserId &&
    (server.ownerId === currentUserId ||
      (userPermissions(rolesByServer[server.id], currentUserId) &
        PERM_MUTE_MEMBERS) !==
        0);
  const canDeafenIn = (server: Server) =>
    !!currentUserId &&
    (server.ownerId === currentUserId ||
      (userPermissions(rolesByServer[server.id], currentUserId) &
        PERM_DEAFEN_MEMBERS) !==
        0);
  const canLiftDeafenIn = (
    server: Server,
    m: { id: string; deafenedBy?: string },
  ) =>
    !moderatorDeafened(m) ||
    m.deafenedBy !== server.ownerId ||
    currentUserId === server.ownerId;

  return {
    canInviteIn,
    canManageChannelsIn,
    outranksIn,
    canMuteIn,
    canDeafenIn,
    canLiftDeafenIn,
  };
}
