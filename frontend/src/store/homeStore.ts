import { create } from "zustand";

import { homeAPI, HomeOverview } from "../api/home";

interface HomeState {
  overview: HomeOverview | null;
  isLoading: boolean;
  fetch: () => Promise<void>;
  clear: () => void;
}

let inflight: Promise<void> | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

export const useHomeStore = create<HomeState>()((set) => ({
  overview: null,
  isLoading: false,

  fetch: async () => {
    if (inflight) return inflight;
    set({ isLoading: true });
    inflight = homeAPI
      .overview()
      .then((overview) => set({ overview, isLoading: false }))
      .catch((err) => {
        console.warn("[home] overview fetch failed:", err);
        set({ isLoading: false });
      })
      .finally(() => {
        inflight = null;
      });
    return inflight;
  },

  clear: () => set({ overview: null }),
}));

export const scheduleHomeRefresh = (delayMs = 400) => {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = null;
    void useHomeStore.getState().fetch();
  }, delayMs);
};
