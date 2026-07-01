import { create } from "zustand";

export const streamViewerKey = (
  serverId: string,
  channelId: string,
  streamerId: string,
) => `${serverId}:${channelId}:${streamerId}`;

interface StreamViewersState {
  counts: Record<string, number>;
  setCount: (
    serverId: string,
    channelId: string,
    streamerId: string,
    count: number,
  ) => void;
  resetForServer: (
    serverId: string,
    viewers: { channelId: string; streamerId: string; count: number }[],
  ) => void;
  clearAll: () => void;
}

export const useStreamViewersStore = create<StreamViewersState>()((set) => ({
  counts: {},

  setCount: (serverId, channelId, streamerId, count) =>
    set((state) => ({
      counts: {
        ...state.counts,
        [streamViewerKey(serverId, channelId, streamerId)]: count,
      },
    })),

  resetForServer: (serverId, viewers) =>
    set((state) => {
      const prefix = `${serverId}:`;
      const next: Record<string, number> = {};
      for (const [key, count] of Object.entries(state.counts)) {
        if (!key.startsWith(prefix)) next[key] = count;
      }
      for (const v of viewers) {
        next[streamViewerKey(serverId, v.channelId, v.streamerId)] = v.count;
      }
      return { counts: next };
    }),

  clearAll: () => set({ counts: {} }),
}));
