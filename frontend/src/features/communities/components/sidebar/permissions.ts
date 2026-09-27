import React from "react";
import type { Community } from "../../../../shared/types/community";
import {
  PERM_MANAGE_CHANNELS,
  PERM_MANAGE_INVITES,
  PERM_DEAFEN_MEMBERS,
  PERM_DISCONNECT_MEMBERS,
  PERM_MUTE_MEMBERS,
  topRoleFor,
  useRolesStore,
  userPermissions,
} from "../../store/rolesStore";

export interface SidebarPermissions {
  canInviteIn: (community: Community) => boolean;
  canManageChannelsIn: (community: Community) => boolean;
  outranksIn: (community: Community, memberId: string) => boolean;
  canMuteIn: (community: Community) => boolean;
  canDeafenIn: (community: Community) => boolean;
  canDisconnectIn: (community: Community) => boolean;
  canLiftDeafenIn: (
    community: Community,
    m: { id: string; deafenedBy?: string },
  ) => boolean;
}

export const moderatorDeafened = (m: { id: string; deafenedBy?: string }) =>
  !!m.deafenedBy && m.deafenedBy !== m.id;

export function useSidebarPermissions(
  currentUserId: string | undefined,
  openCommunityId: string | null,
): SidebarPermissions {
  const rolesByCommunity = useRolesStore((st) => st.byCommunity);
  const fetchRoles = useRolesStore((st) => st.fetch);
  React.useEffect(() => {
    if (openCommunityId) void fetchRoles(openCommunityId).catch(() => {});
  }, [openCommunityId, fetchRoles]);

  const canInviteIn = (community: Community) =>
    !!currentUserId &&
    (community.ownerId === currentUserId ||
      (userPermissions(rolesByCommunity[community.id], currentUserId) &
        PERM_MANAGE_INVITES) !==
        0);
  const canManageChannelsIn = (community: Community) =>
    !!currentUserId &&
    (community.ownerId === currentUserId ||
      (userPermissions(rolesByCommunity[community.id], currentUserId) &
        PERM_MANAGE_CHANNELS) !==
        0);
  const outranksIn = (community: Community, memberId: string) => {
    if (!currentUserId) return false;
    if (community.ownerId === currentUserId) return true;
    if (memberId === community.ownerId) return false;
    const data = rolesByCommunity[community.id];
    const myTop = topRoleFor(data, currentUserId)?.position;
    if (myTop === undefined) return false;
    const targetTop = topRoleFor(data, memberId)?.position;
    return targetTop === undefined || targetTop < myTop;
  };
  const canMuteIn = (community: Community) =>
    !!currentUserId &&
    (community.ownerId === currentUserId ||
      (userPermissions(rolesByCommunity[community.id], currentUserId) &
        PERM_MUTE_MEMBERS) !==
        0);
  const canDeafenIn = (community: Community) =>
    !!currentUserId &&
    (community.ownerId === currentUserId ||
      (userPermissions(rolesByCommunity[community.id], currentUserId) &
        PERM_DEAFEN_MEMBERS) !==
        0);
  const canDisconnectIn = (community: Community) =>
    !!currentUserId &&
    (community.ownerId === currentUserId ||
      (userPermissions(rolesByCommunity[community.id], currentUserId) &
        PERM_DISCONNECT_MEMBERS) !==
        0);
  const canLiftDeafenIn = (
    community: Community,
    m: { id: string; deafenedBy?: string },
  ) =>
    !moderatorDeafened(m) ||
    m.deafenedBy !== community.ownerId ||
    currentUserId === community.ownerId;

  return {
    canInviteIn,
    canManageChannelsIn,
    outranksIn,
    canMuteIn,
    canDeafenIn,
    canDisconnectIn,
    canLiftDeafenIn,
  };
}
