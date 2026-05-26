import { create } from "zustand";

interface UnreadState {
  byChannel: Record<string, number>;
  byDm: Record<string, number>;
  channelToServer: Record<string, string>;
  setAll: (
    channels: Record<string, number>,
    dms: Record<string, number>,
    channelToServer?: Record<string, string>,
  ) => void;
  bumpChannel: (channelId: string, serverId?: string) => void;
  bumpDm: (friendId: string) => void;
  resetChannel: (channelId: string) => void;
  resetDm: (friendId: string) => void;
  clearAll: () => void;
  serverUnread: (serverId: string) => number;
}

export const useUnreadStore = create<UnreadState>()((set, get) => ({
  byChannel: {},
  byDm: {},
  channelToServer: {},

  setAll: (channels, dms, channelToServer) =>
    set({
      byChannel: channels,
      byDm: dms,
      channelToServer: channelToServer ?? {},
    }),

  bumpChannel: (channelId, serverId) =>
    set((state) => ({
      byChannel: {
        ...state.byChannel,
        [channelId]: (state.byChannel[channelId] ?? 0) + 1,
      },
      channelToServer: serverId
        ? { ...state.channelToServer, [channelId]: serverId }
        : state.channelToServer,
    })),

  bumpDm: (friendId) =>
    set((state) => ({
      byDm: { ...state.byDm, [friendId]: (state.byDm[friendId] ?? 0) + 1 },
    })),

  resetChannel: (channelId) =>
    set((state) => {
      const hasUnread = channelId in state.byChannel;
      const hasMapping = channelId in state.channelToServer;
      if (!hasUnread && !hasMapping) return state;
      const { [channelId]: _gone, ...rest } = state.byChannel;
      const { [channelId]: _g2, ...restMap } = state.channelToServer;
      return { byChannel: rest, channelToServer: restMap };
    }),

  resetDm: (friendId) =>
    set((state) => {
      if (!(friendId in state.byDm)) return state;
      const { [friendId]: _gone, ...rest } = state.byDm;
      return { byDm: rest };
    }),

  clearAll: () => set({ byChannel: {}, byDm: {}, channelToServer: {} }),

  serverUnread: (serverId) => {
    const { byChannel, channelToServer } = get();
    let total = 0;
    for (const [channelId, count] of Object.entries(byChannel)) {
      if (channelToServer[channelId] === serverId) total += count;
    }
    return total;
  },
}));
