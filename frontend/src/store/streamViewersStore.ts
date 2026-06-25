import { create } from "zustand";

export const streamViewerKey = (channelId: string, streamerId: string) =>
  `${channelId}:${streamerId}`;

interface StreamViewersState {
  counts: Record<string, number>;
  setCount: (channelId: string, streamerId: string, count: number) => void;
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

  clearAll: () => set({ counts: {} }),
}));
