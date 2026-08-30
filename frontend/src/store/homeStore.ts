import { create } from "zustand";

import { homeAPI, HomeOverview } from "../api/home";

interface HomeState {
  overview: HomeOverview | null;
  isLoading: boolean;
  failed: boolean;
  fetch: (force?: boolean) => Promise<void>;
  clear: () => void;
  hideFriendActivity: (userId: string) => void;
}

let inflight: Promise<void> | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let generation = 0;

export const useHomeStore = create<HomeState>()((set, get) => ({
  overview: null,
  isLoading: false,
  failed: false,

  fetch: async (force = false) => {
    if (inflight) {
      if (!force) return inflight;
      const pending = inflight;
      return pending.catch(() => {}).then(() => get().fetch());
    }
    const gen = generation;
    set({ isLoading: true });
    const request: Promise<void> = homeAPI
      .overview()
      .then((overview) => {
        if (gen === generation) set({ overview, isLoading: false, failed: false });
      })
      .catch((err) => {
        if (gen !== generation) return;
        console.warn("[home] overview fetch failed:", err);
        set({ isLoading: false, failed: true });
      })
      .finally(() => {
        if (inflight === request) inflight = null;
      });
    inflight = request;
    return request;
  },

  hideFriendActivity: (userId) =>
    set((s) =>
      s.overview && s.overview.activeFriends.some((f) => f.userId === userId)
        ? { overview: { ...s.overview, activeFriends: s.overview.activeFriends.filter((f) => f.userId !== userId) } }
        : s,
    ),

  clear: () => {
    generation += 1;
    inflight = null;
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    set({ overview: null, isLoading: false, failed: false });
  },
}));

export const scheduleHomeRefresh = (delayMs = 400) => {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = null;
    void useHomeStore.getState().fetch(true);
  }, delayMs);
};
