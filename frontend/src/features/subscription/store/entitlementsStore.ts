import { create } from "zustand";

import { entitlementsAPI } from "../api/entitlements";
import { DEFAULT_BADGE_STYLE } from "../lib/plans";
import type { BadgeStyle, Tier } from "../lib/plans";

interface EntitlementsState {
  tier: Tier;
  badgeStyle: BadgeStyle;
  loaded: boolean;
  fetch: () => Promise<void>;
  applyTier: (tier: Tier) => void;
  applyBadgeStyle: (style: BadgeStyle) => void;
  setBadgeStyle: (style: BadgeStyle) => Promise<void>;
  clear: () => void;
}

let generation = 0;

export const useEntitlementsStore = create<EntitlementsState>()((set, get) => ({
  tier: "free",
  badgeStyle: DEFAULT_BADGE_STYLE,
  loaded: false,

  fetch: async () => {
    const gen = generation;
    try {
      const mine = await entitlementsAPI.getMine();
      if (gen !== generation) return;
      set({ tier: mine.tier, badgeStyle: mine.badgeStyle, loaded: true });
    } catch (err) {
      if (gen !== generation) return;
      console.warn("[entitlements] failed to load:", err);
      set({ loaded: true });
    }
  },

  applyTier: (tier) => set({ tier }),
  applyBadgeStyle: (badgeStyle) => set({ badgeStyle }),

  setBadgeStyle: async (style) => {
    const previous = get().badgeStyle;
    set({ badgeStyle: style });
    try {
      const saved = await entitlementsAPI.setBadgeStyle(style);
      set({ badgeStyle: saved });
    } catch (err) {
      set({ badgeStyle: previous });
      throw err;
    }
  },

  clear: () => {
    generation += 1;
    set({ tier: "free", badgeStyle: DEFAULT_BADGE_STYLE, loaded: false });
  },
}));
