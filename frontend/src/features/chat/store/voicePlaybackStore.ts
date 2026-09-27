import { create } from "zustand";

export const VOICE_SPEEDS = [1, 1.5, 2] as const;
export type VoiceSpeed = (typeof VOICE_SPEEDS)[number];

const SPEED_KEY = "rift.voiceSpeed";

const readSpeed = (): VoiceSpeed => {
  try {
    const raw = Number(localStorage.getItem(SPEED_KEY));
    return (VOICE_SPEEDS as readonly number[]).includes(raw) ? (raw as VoiceSpeed) : 1;
  } catch {
    return 1;
  }
};

export interface VoicePlayerHandle {
  element: () => HTMLElement | null;
  autoplayable: () => boolean;
  play: () => void;
}

const players = new Map<string, VoicePlayerHandle>();

interface VoicePlaybackState {
  currentId: string | null;
  speed: VoiceSpeed;
  setCurrent: (id: string | null) => void;
  release: (id: string) => void;
  cycleSpeed: () => void;
}

export const useVoicePlaybackStore = create<VoicePlaybackState>()((set, get) => ({
  currentId: null,
  speed: readSpeed(),
  setCurrent: (id) => set({ currentId: id }),
  release: (id) => {
    if (get().currentId === id) set({ currentId: null });
  },
  cycleSpeed: () => {
    const index = VOICE_SPEEDS.indexOf(get().speed);
    const next = VOICE_SPEEDS[(index + 1) % VOICE_SPEEDS.length];
    try {
      localStorage.setItem(SPEED_KEY, String(next));
    } catch {}
    set({ speed: next });
  },
}));

export function registerVoicePlayer(id: string, handle: VoicePlayerHandle): () => void {
  players.set(id, handle);
  return () => {
    if (players.get(id) === handle) players.delete(id);
  };
}

export function nextAutoplayAfter(id: string): VoicePlayerHandle | null {
  const current = players.get(id)?.element();
  if (!current) return null;
  let best: { handle: VoicePlayerHandle; el: HTMLElement } | null = null;
  players.forEach((handle, otherId) => {
    if (otherId === id || !handle.autoplayable()) return;
    const el = handle.element();
    if (!el || !(current.compareDocumentPosition(el) & Node.DOCUMENT_POSITION_FOLLOWING)) return;
    if (!best || el.compareDocumentPosition(best.el) & Node.DOCUMENT_POSITION_FOLLOWING) {
      best = { handle, el };
    }
  });
  return (best as { handle: VoicePlayerHandle } | null)?.handle ?? null;
}
