import { create } from "zustand";

interface VoiceSessionState {
  startedAt: Record<string, string>;
  seq: number;
  touched: Record<string, number>;
  replaceForChannels: (
    channelIds: string[],
    sessions: { channelId: string; startedAt: string }[],
    since?: number,
  ) => void;
  begin: (channelId: string, startedAt: string) => void;
  end: (channelId: string) => void;
  clearAll: () => void;
}

export const useVoiceSessionStore = create<VoiceSessionState>()((set) => ({
  startedAt: {},
  seq: 0,
  touched: {},
  replaceForChannels: (channelIds, sessions, since = Number.POSITIVE_INFINITY) =>
    set((state) => {
      const fresh = (id: string) => (state.touched[id] ?? 0) <= since;
      const next = { ...state.startedAt };
      for (const id of channelIds) if (fresh(id)) delete next[id];
      for (const s of sessions) if (fresh(s.channelId)) next[s.channelId] = s.startedAt;
      return { startedAt: next };
    }),
  begin: (channelId, startedAt) =>
    set((state) => {
      const seq = state.seq + 1;
      const touched = { ...state.touched, [channelId]: seq };
      if (state.startedAt[channelId]) return { seq, touched };
      return { seq, touched, startedAt: { ...state.startedAt, [channelId]: startedAt } };
    }),
  end: (channelId) =>
    set((state) => {
      const seq = state.seq + 1;
      const touched = { ...state.touched, [channelId]: seq };
      if (!state.startedAt[channelId]) return { seq, touched };
      const { [channelId]: _gone, ...rest } = state.startedAt;
      return { seq, touched, startedAt: rest };
    }),
  clearAll: () => set({ startedAt: {}, touched: {} }),
}));
