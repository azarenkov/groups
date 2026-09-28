import { create } from "zustand";

import type { VoiceRecording } from "../lib/voiceRecorder";

interface VoiceDraftState {
  failed: Record<string, VoiceRecording>;
  keep: (key: string, recording: VoiceRecording) => void;
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
