import { create } from "zustand";

interface UserNamesState {
  overrides: Record<string, string>;
  syncTick: number;
  setName: (userId: string, name: string) => void;
  bumpSyncTick: () => void;
  clear: () => void;
}

export const useUserNamesStore = create<UserNamesState>()((set) => ({
  overrides: {},
  syncTick: 0,
  setName: (userId, name) =>
    set((state) => ({ overrides: { ...state.overrides, [userId]: name } })),
  bumpSyncTick: () => set((state) => ({ syncTick: state.syncTick + 1 })),
  clear: () => set({ overrides: {} }),
}));
