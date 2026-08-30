import { create } from "zustand";

import { homeAPI, HomeOverview } from "../api/home";

interface HomeState {
  overview: HomeOverview | null;
  isLoading: boolean;
  failed: boolean;
  hiddenActivity: Record<string, string>;
  fetch: (force?: boolean) => Promise<void>;
  clear: () => void;
  hideFriendActivity: (userId: string, version: string) => void;
  unhideFriendActivity: (userId: string, version: string) => void;
}

const applyHidden = (overview: HomeOverview, hidden: Record<string, string>): HomeOverview =>
  Object.keys(hidden).length === 0
    ? overview
    : { ...overview, activeFriends: overview.activeFriends.filter((f) => !(f.userId in hidden)) };

let inflight: Promise<void> | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let generation = 0;

export const useHomeStore = create<HomeState>()((set, get) => ({
  overview: null,
  isLoading: false,
  failed: false,
  hiddenActivity: {},

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
        if (gen === generation) set({ overview: applyHidden(overview, get().hiddenActivity), isLoading: false, failed: false });
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

  hideFriendActivity: (userId, version) =>
    set((s) => {
      const current = s.hiddenActivity[userId];
      if (current !== undefined && current > version) return s;
      const hiddenActivity = { ...s.hiddenActivity, [userId]: version };
      return {
        hiddenActivity,
        overview: s.overview ? applyHidden(s.overview, hiddenActivity) : s.overview,
      };
    }),

  unhideFriendActivity: (userId, version) =>
    set((s) => {
      const current = s.hiddenActivity[userId];
      if (current === undefined || current > version) return s;
      const { [userId]: _gone, ...hiddenActivity } = s.hiddenActivity;
      return { hiddenActivity };
    }),

  clear: () => {
    generation += 1;
    inflight = null;
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    set({ overview: null, isLoading: false, failed: false, hiddenActivity: {} });
  },
}));

export const scheduleHomeRefresh = (delayMs = 400) => {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = null;
    void useHomeStore.getState().fetch(true);
  }, delayMs);
};
