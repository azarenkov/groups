import { create } from "zustand";

import { serverAPI } from "../api/server";

export interface ServerRole {
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
export const PERM_MANAGE_SERVER = 32;

interface ServerRolesData {
  roles: ServerRole[];
  memberRoles: Record<string, string[]>;
}

interface RolesState {
  byServer: Record<string, ServerRolesData>;
  fetch: (serverId: string) => Promise<void>;
  clearAll: () => void;
}

const fetchGens: Record<string, number> = {};

export const useRolesStore = create<RolesState>()((set) => ({
  byServer: {},

  fetch: async (serverId) => {
    const gen = (fetchGens[serverId] = (fetchGens[serverId] ?? 0) + 1);
    const data = await serverAPI.listRoles(serverId);
    if (fetchGens[serverId] !== gen) return;
    const memberRoles: Record<string, string[]> = {};
    for (const a of data.assignments) {
      (memberRoles[a.userId] ??= []).push(a.roleId);
    }
    set((s) => ({
      byServer: {
        ...s.byServer,
        [serverId]: { roles: data.roles, memberRoles },
      },
    }));
  },

  clearAll: () => {
    for (const k of Object.keys(fetchGens)) delete fetchGens[k];
    set({ byServer: {} });
  },
}));

export function topRoleFor(
  data: ServerRolesData | undefined,
  userId: string,
): ServerRole | null {
  if (!data) return null;
  const ids = data.memberRoles[userId];
  if (!ids || ids.length === 0) return null;
  let top: ServerRole | null = null;
  for (const role of data.roles) {
    if (!ids.includes(role.id)) continue;
    if (!top || role.position > top.position) top = role;
  }
  return top;
}

export function userPermissions(
  data: ServerRolesData | undefined,
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
