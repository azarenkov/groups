import { create } from "zustand";

export const streamViewerKey = (
  communityId: string,
  channelId: string,
  streamerId: string,
) => `${communityId}:${channelId}:${streamerId}`;

interface StreamViewersState {
  counts: Record<string, number>;
  setCount: (
    communityId: string,
    channelId: string,
    streamerId: string,
    count: number,
  ) => void;
  resetForCommunity: (
    communityId: string,
    viewers: { channelId: string; streamerId: string; count: number }[],
  ) => void;
  clearAll: () => void;
}

export const useStreamViewersStore = create<StreamViewersState>()((set) => ({
  counts: {},

  setCount: (communityId, channelId, streamerId, count) =>
    set((state) => ({
      counts: {
        ...state.counts,
        [streamViewerKey(communityId, channelId, streamerId)]: count,
      },
    })),

  resetForCommunity: (communityId, viewers) =>
    set((state) => {
      const prefix = `${communityId}:`;
      const next: Record<string, number> = {};
      for (const [key, count] of Object.entries(state.counts)) {
        if (!key.startsWith(prefix)) next[key] = count;
      }
      for (const v of viewers) {
        next[streamViewerKey(communityId, v.channelId, v.streamerId)] = v.count;
      }
      return { counts: next };
    }),

  clearAll: () => set({ counts: {} }),
}));
