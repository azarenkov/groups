import { create } from "zustand";

interface UnreadState {
  byChannel: Record<string, number>;
  byDm: Record<string, number>;
  setAll: (
    channels: Record<string, number>,
    dms: Record<string, number>,
  ) => void;
  bumpChannel: (channelId: string) => void;
  bumpDm: (friendId: string) => void;
  resetChannel: (channelId: string) => void;
  resetDm: (friendId: string) => void;
  clearAll: () => void;
}

export const useUnreadStore = create<UnreadState>()((set) => ({
  byChannel: {},
  byDm: {},

  setAll: (channels, dms) => set({ byChannel: channels, byDm: dms }),

  bumpChannel: (channelId) =>
    set((state) => ({
      byChannel: {
        ...state.byChannel,
        [channelId]: (state.byChannel[channelId] ?? 0) + 1,
      },
    })),

  bumpDm: (friendId) =>
    set((state) => ({
      byDm: { ...state.byDm, [friendId]: (state.byDm[friendId] ?? 0) + 1 },
    })),

  resetChannel: (channelId) =>
    set((state) => {
      if (!(channelId in state.byChannel)) return state;
      const { [channelId]: _gone, ...rest } = state.byChannel;
      return { byChannel: rest };
    }),

  resetDm: (friendId) =>
    set((state) => {
      if (!(friendId in state.byDm)) return state;
      const { [friendId]: _gone, ...rest } = state.byDm;
      return { byDm: rest };
    }),

  clearAll: () => set({ byChannel: {}, byDm: {} }),
}));
