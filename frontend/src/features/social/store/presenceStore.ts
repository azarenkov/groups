import { create } from "zustand";

interface PresenceState {
  online: Record<string, true>;
  selfUserId: string | null;
  selfOnline: boolean;
  setOnline: (userId: string) => void;
  applySnapshot: (userId: string) => void;
  setOffline: (userId: string) => void;
  setBulk: (userIds: string[]) => void;
  setSelf: (userId: string, online: boolean) => void;
  clearAll: () => void;
}

const presenceSeq = new Map<string, number>();
let presenceEpoch = 0;
const bumpPresence = (userId: string) => presenceSeq.set(userId, (presenceSeq.get(userId) ?? 0) + 1);
const resetPresenceGenerations = () => {
  presenceEpoch += 1;
  presenceSeq.clear();
};

export const presenceGeneration = (userId: string): string => `${presenceEpoch}:${presenceSeq.get(userId) ?? 0}`;

export const usePresenceStore = create<PresenceState>()((set) => ({
  online: {},
  selfUserId: null,
  selfOnline: true,

  setOnline: (userId) => {
    bumpPresence(userId);
    set((state) =>
      state.online[userId]
        ? state
        : { online: { ...state.online, [userId]: true } },
    );
  },

  applySnapshot: (userId) =>
    set((state) =>
      state.online[userId]
        ? state
        : { online: { ...state.online, [userId]: true } },
    ),

  setOffline: (userId) => {
    bumpPresence(userId);
    set((state) => {
      if (!state.online[userId]) return state;
      const { [userId]: _gone, ...rest } = state.online;
      return { online: rest };
    });
  },

  setBulk: (userIds) =>
    set(() => {
      resetPresenceGenerations();
      const next: Record<string, true> = {};
      for (const id of userIds) next[id] = true;
      return { online: next };
    }),

  setSelf: (userId, online) => set({ selfUserId: userId, selfOnline: online }),

  clearAll: () => {
    resetPresenceGenerations();
    set({ online: {} });
  },
}));
