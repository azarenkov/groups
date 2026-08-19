import { create } from "zustand";

interface RenameStamp {
  ms: number;
  frac: string;
}

interface UserNamesState {
  overrides: Record<string, string>;
  renamedAt: Record<string, RenameStamp>;
  syncTick: number;
  applyRename: (
    userId: string,
    name: string,
    occurredAt: string | undefined,
  ) => boolean;
  bumpSyncTick: () => void;
  clear: () => void;
}

const parseStamp = (occurredAt: string | undefined): RenameStamp => {
  if (occurredAt) {
    const ms = Date.parse(occurredAt);
    if (!Number.isNaN(ms)) {
      const frac = occurredAt.match(/\.(\d+)/)?.[1] ?? "";
      return { ms, frac: frac.padEnd(9, "0").slice(0, 9) };
    }
  }
  return { ms: Date.now(), frac: "0".repeat(9) };
};

const isNewer = (stamp: RenameStamp, last: RenameStamp | undefined): boolean =>
  last === undefined ||
  stamp.ms > last.ms ||
  (stamp.ms === last.ms && stamp.frac > last.frac);

export const useUserNamesStore = create<UserNamesState>()((set, get) => ({
  overrides: {},
  renamedAt: {},
  syncTick: 0,
  applyRename: (userId, name, occurredAt) => {
    const stamp = parseStamp(occurredAt);
    if (!isNewer(stamp, get().renamedAt[userId])) return false;
    set((state) => ({
      overrides: { ...state.overrides, [userId]: name },
      renamedAt: { ...state.renamedAt, [userId]: stamp },
    }));
    return true;
  },
  bumpSyncTick: () => set((state) => ({ syncTick: state.syncTick + 1 })),
  clear: () => set({ overrides: {} }),
}));
