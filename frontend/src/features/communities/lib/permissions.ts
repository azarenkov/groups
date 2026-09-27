export const PERMISSIONS = [
  "manageChannels",
  "manageInvites",
  "manageMessages",
  "muteMembers",
  "deafenMembers",
  "disconnectMembers",
  "manageRoles",
  "manageCommunity",
  "kickMembers",
] as const;

export type Permission = (typeof PERMISSIONS)[number];

export const PERMISSION_LABEL_KEYS: Record<Permission, string> = {
  manageChannels: "permManageChannels",
  manageInvites: "permManageInvites",
  manageMessages: "permManageMessages",
  muteMembers: "permMuteMembers",
  deafenMembers: "permDeafenMembers",
  disconnectMembers: "permDisconnectMembers",
  manageRoles: "permManageRoles",
  manageCommunity: "permManageCommunity",
  kickMembers: "permKickMembers",
};

const LEGACY_BITS: Record<Permission, number> = {
  manageChannels: 1,
  manageInvites: 2,
  manageMessages: 4,
  muteMembers: 8,
  manageRoles: 16,
  manageCommunity: 32,
  kickMembers: 64,
  deafenMembers: 128,
  disconnectMembers: 256,
};

export function isPermission(value: unknown): value is Permission {
  return typeof value === "string" && (PERMISSIONS as readonly string[]).includes(value);
}

export function readPermissions(list: unknown, legacyMask?: unknown): Permission[] {
  if (Array.isArray(list)) return normalize(list.filter(isPermission));
  if (typeof legacyMask === "number") {
    return PERMISSIONS.filter((p) => (legacyMask & LEGACY_BITS[p]) !== 0);
  }
  return [];
}

export function normalize(set: readonly Permission[]): Permission[] {
  return PERMISSIONS.filter((p) => set.includes(p));
}

export const hasPermission = (set: readonly Permission[], permission: Permission): boolean =>
  set.includes(permission);

export const union = (a: readonly Permission[], b: readonly Permission[]): Permission[] =>
  normalize([...a, ...b]);

export const without = (a: readonly Permission[], b: readonly Permission[]): Permission[] =>
  normalize(a.filter((p) => !b.includes(p)));

export const isSubset = (a: readonly Permission[], b: readonly Permission[]): boolean =>
  a.every((p) => b.includes(p));
