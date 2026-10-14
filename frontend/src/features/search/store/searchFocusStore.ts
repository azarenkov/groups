import { create } from "zustand";

export type SearchFocusMode = "people" | "invite";

interface SearchFocusState {
  request: { mode: SearchFocusMode; seq: number } | null;
  focus: (mode: SearchFocusMode) => void;
}

export const useSearchFocusStore = create<SearchFocusState>()((set) => ({
  request: null,
  focus: (mode) => set((s) => ({ request: { mode, seq: (s.request?.seq ?? 0) + 1 } })),
}));
