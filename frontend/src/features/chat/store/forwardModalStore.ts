import { create } from "zustand";

import type { ForwardSourceRef } from "../api/forward";

interface ForwardModalState {
  source: ForwardSourceRef | null;
  opening: number;
  open: (source: ForwardSourceRef) => void;
  close: () => void;
}

export const useForwardModalStore = create<ForwardModalState>()((set) => ({
  source: null,
  opening: 0,
  open: (source) => set((s) => ({ source, opening: s.opening + 1 })),
  close: () => set({ source: null }),
}));

export const openForward = (source: ForwardSourceRef) =>
  useForwardModalStore.getState().open(source);
