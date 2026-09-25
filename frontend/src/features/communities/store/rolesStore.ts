import { create } from "zustand";

import { communityAPI } from "../api/community";

export interface CommunityRole {
  id: string;
  name: string;
  color: string;
  permissions: number;
  position: number;
}

export const PERM_MANAGE_CHANNELS = 1;
export const PERM_MANAGE_INVITES = 2;
export const PERM_MANAGE_MESSAGES = 4;
export const PERM_MUTE_MEMBERS = 8;
export const PERM_MANAGE_ROLES = 16;
export const PERM_MANAGE_COMMUNITY = 32;
export const PERM_KICK_MEMBERS = 64;
export const PERM_DEAFEN_MEMBERS = 128;
export const PERM_DISCONNECT_MEMBERS = 256;

interface CommunityRolesData {
  roles: CommunityRole[];
  memberRoles: Record<string, string[]>;
}

interface RolesState {
  byCommunity: Record<string, CommunityRolesData>;
  fetch: (communityId: string) => Promise<void>;
  removeMember: (communityId: string, userId: string) => void;
  clearCommunity: (communityId: string) => void;
  isTracked: (communityId: string) => boolean;
  trackedCommunities: () => string[];
  clearAll: () => void;
}

const fetchGens: Record<string, number> = {};
let nextGen = 0;

export const useRolesStore = create<RolesState>()((set) => ({
  byCommunity: {},

  fetch: async (communityId) => {
    const gen = (fetchGens[communityId] = ++nextGen);
    let data: Awaited<ReturnType<typeof communityAPI.listRoles>>;
    try {
      data = await communityAPI.listRoles(communityId);
    } catch (err) {
      if (fetchGens[communityId] === gen) delete fetchGens[communityId];
      throw err;
    }
    if (fetchGens[communityId] !== gen) return;
    const memberRoles: Record<string, string[]> = {};
    for (const a of data.assignments) {
      (memberRoles[a.userId] ??= []).push(a.roleId);
    }
    set((s) => ({
      byCommunity: {
        ...s.byCommunity,
        [communityId]: { roles: data.roles, memberRoles },
      },
    }));
  },

  clearCommunity: (communityId) =>
    set((s) => {
      delete fetchGens[communityId];
      if (!s.byCommunity[communityId]) return s;
      const byCommunity = { ...s.byCommunity };
      delete byCommunity[communityId];
      return { byCommunity };
    }),

  isTracked: (communityId) => fetchGens[communityId] !== undefined,

  trackedCommunities: () => Object.keys(fetchGens),

  removeMember: (communityId, userId) =>
    set((s) => {
      if (fetchGens[communityId] !== undefined) fetchGens[communityId] = ++nextGen;
      const data = s.byCommunity[communityId];
      if (!data || !data.memberRoles[userId]) return s;
      const memberRoles = { ...data.memberRoles };
      delete memberRoles[userId];
      return {
        byCommunity: { ...s.byCommunity, [communityId]: { ...data, memberRoles } },
      };
    }),

  clearAll: () => {
    for (const k of Object.keys(fetchGens)) delete fetchGens[k];
    set({ byCommunity: {} });
  },
}));

export function topRoleFor(
  data: CommunityRolesData | undefined,
  userId: string,
): CommunityRole | null {
  if (!data) return null;
  const ids = data.memberRoles[userId];
  if (!ids || ids.length === 0) return null;
  let top: CommunityRole | null = null;
  for (const role of data.roles) {
    if (!ids.includes(role.id)) continue;
    if (!top || role.position > top.position) top = role;
  }
  return top;
}

export function userPermissions(
  data: CommunityRolesData | undefined,
  userId: string,
): number {
  if (!data) return 0;
  const ids = data.memberRoles[userId];
  if (!ids) return 0;
  let perms = 0;
  for (const role of data.roles) {
    if (ids.includes(role.id)) perms |= role.permissions;
  }
  return perms;
}
