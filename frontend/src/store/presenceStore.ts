import { create } from "zustand";

interface PresenceState {
  online: Record<string, true>;
  selfUserId: string | null;
  selfOnline: boolean;
  setOnline: (userId: string) => void;
  setOffline: (userId: string) => void;
  setBulk: (userIds: string[]) => void;
  setSelf: (userId: string, online: boolean) => void;
  clearAll: () => void;
}

export const usePresenceStore = create<PresenceState>()((set) => ({
  online: {},
  selfUserId: null,
  selfOnline: true,

  setOnline: (userId) =>
    set((state) =>
      state.online[userId]
        ? state
        : { online: { ...state.online, [userId]: true } },
    ),

  setOffline: (userId) =>
    set((state) => {
      if (!state.online[userId]) return state;
      const { [userId]: _gone, ...rest } = state.online;
      return { online: rest };
    }),

  setBulk: (userIds) =>
    set(() => {
      const next: Record<string, true> = {};
      for (const id of userIds) next[id] = true;
      return { online: next };
    }),

  setSelf: (userId, online) => set({ selfUserId: userId, selfOnline: online }),

  clearAll: () => set({ online: {} }),
}));
