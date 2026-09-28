import { create } from "zustand";

import type { MediaRecording } from "../lib/recording";

interface VoiceDraftState {
  failed: Record<string, MediaRecording>;
  keep: (key: string, recording: MediaRecording) => void;
  drop: (key: string) => void;
}

export const useVoiceDraftStore = create<VoiceDraftState>()((set) => ({
  failed: {},
  keep: (key, recording) => set((s) => ({ failed: { ...s.failed, [key]: recording } })),
  drop: (key) =>
    set((s) => {
      if (!(key in s.failed)) return s;
      const next = { ...s.failed };
      delete next[key];
      return { failed: next };
    }),
}));
