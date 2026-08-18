import { create } from "zustand";

interface UserNamesState {
  overrides: Record<string, string>;
  renamedAt: Record<string, number>;
  syncTick: number;
  applyRename: (userId: string, name: string, occurredAt: number) => boolean;
  bumpSyncTick: () => void;
  clear: () => void;
}

export const useUserNamesStore = create<UserNamesState>()((set, get) => ({
  overrides: {},
  renamedAt: {},
  syncTick: 0,
  applyRename: (userId, name, occurredAt) => {
    const last = get().renamedAt[userId];
    if (last !== undefined && occurredAt <= last) return false;
    set((state) => ({
      overrides: { ...state.overrides, [userId]: name },
      renamedAt: { ...state.renamedAt, [userId]: occurredAt },
    }));
    return true;
  },
  bumpSyncTick: () => set((state) => ({ syncTick: state.syncTick + 1 })),
  clear: () => set({ overrides: {} }),
}));
