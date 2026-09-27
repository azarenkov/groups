import { create } from "zustand";

import { communityAPI } from "../api/community";
import type { Permission } from "../lib/permissions";
import { union } from "../lib/permissions";

export interface CommunityRole {
  id: string;
  name: string;
  color: string;
  permissions: Permission[];
  position: number;
}

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
): Permission[] {
  if (!data) return [];
  const ids = data.memberRoles[userId];
  if (!ids) return [];
  let perms: Permission[] = [];
  for (const role of data.roles) {
    if (ids.includes(role.id)) perms = union(perms, role.permissions);
  }
  return perms;
}
