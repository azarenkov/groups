import { create } from "zustand";

interface VoiceChannelState {
  name: string | null;
  leaveFn: (() => Promise<void>) | null;
  setSession: (name: string | null, leave: (() => Promise<void>) | null) => void;
}

export const useVoiceChannelStore = create<VoiceChannelState>()((set) => ({
  name: null,
  leaveFn: null,
  setSession: (name, leave) => set({ name, leaveFn: name ? leave : null }),
}));

export async function leaveVoiceChannel(): Promise<boolean> {
  const { leaveFn } = useVoiceChannelStore.getState();
  if (!leaveFn) return false;
  useVoiceChannelStore.getState().setSession(null, null);
  try {
    await leaveFn();
  } catch (err) {
    console.warn("[voice] leaving the channel before a call failed:", err);
  }
  return true;
}
