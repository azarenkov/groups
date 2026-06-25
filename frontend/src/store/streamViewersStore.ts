import { create } from "zustand";

export const streamViewerKey = (channelId: string, streamerId: string) =>
  `${channelId}:${streamerId}`;

interface StreamViewersState {
  counts: Record<string, number>;
  setCount: (channelId: string, streamerId: string, count: number) => void;
  resetForChannels: (
    channelIds: string[],
    viewers: { channelId: string; streamerId: string; count: number }[],
  ) => void;
  clearAll: () => void;
}

export const useStreamViewersStore = create<StreamViewersState>()((set) => ({
  counts: {},

  setCount: (channelId, streamerId, count) =>
    set((state) => ({
      counts: {
        ...state.counts,
        [streamViewerKey(channelId, streamerId)]: count,
      },
    })),

  resetForChannels: (channelIds, viewers) =>
    set((state) => {
      const scope = new Set(channelIds);
      const next: Record<string, number> = {};
      for (const [key, count] of Object.entries(state.counts)) {
        const channelId = key.slice(0, key.lastIndexOf(":"));
        if (!scope.has(channelId)) next[key] = count;
      }
      for (const v of viewers) {
        next[streamViewerKey(v.channelId, v.streamerId)] = v.count;
      }
      return { counts: next };
    }),

  clearAll: () => set({ counts: {} }),
}));
