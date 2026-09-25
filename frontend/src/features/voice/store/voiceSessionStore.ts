import { create } from "zustand";

interface VoiceSessionState {
  startedAt: Record<string, string>;
  replaceForChannels: (channelIds: string[], sessions: { channelId: string; startedAt: string }[]) => void;
  begin: (channelId: string, startedAt: string) => void;
  end: (channelId: string) => void;
  clearAll: () => void;
}

export const useVoiceSessionStore = create<VoiceSessionState>()((set) => ({
  startedAt: {},
  replaceForChannels: (channelIds, sessions) =>
    set((state) => {
      const next = { ...state.startedAt };
      for (const id of channelIds) delete next[id];
      for (const s of sessions) next[s.channelId] = s.startedAt;
      return { startedAt: next };
    }),
  begin: (channelId, startedAt) =>
    set((state) =>
      state.startedAt[channelId] ? state : { startedAt: { ...state.startedAt, [channelId]: startedAt } },
    ),
  end: (channelId) =>
    set((state) => {
      if (!state.startedAt[channelId]) return state;
      const { [channelId]: _gone, ...rest } = state.startedAt;
      return { startedAt: rest };
    }),
  clearAll: () => set({ startedAt: {} }),
}));
