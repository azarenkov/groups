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
let badgeInFlight: Promise<void> | null = null;
let desiredBadgeStyle: BadgeStyle | null = null;
let confirmedBadgeStyle: BadgeStyle | null = null;

export const useEntitlementsStore = create<EntitlementsState>()((set, get) => ({
  tier: "free",
  badgeStyle: DEFAULT_BADGE_STYLE,
  loaded: false,

  fetch: async () => {
    const gen = generation;
    try {
      const mine = await entitlementsAPI.getMine();
      if (gen !== generation) return;
      confirmedBadgeStyle = mine.badgeStyle;
      set({ tier: mine.tier, badgeStyle: mine.badgeStyle, loaded: true });
    } catch (err) {
      if (gen !== generation) return;
      console.warn("[entitlements] failed to load:", err);
      set({ loaded: true });
    }
  },

  applyTier: (tier) => set({ tier }),
  applyBadgeStyle: (badgeStyle) => {
    confirmedBadgeStyle = badgeStyle;
    set({ badgeStyle });
  },

  setBadgeStyle: (style) => {
    if (confirmedBadgeStyle === null) confirmedBadgeStyle = get().badgeStyle;
    set({ badgeStyle: style });
    desiredBadgeStyle = style;
    if (badgeInFlight) return badgeInFlight;
    badgeInFlight = (async () => {
      try {
        while (desiredBadgeStyle !== null && desiredBadgeStyle !== confirmedBadgeStyle) {
          const target: BadgeStyle = desiredBadgeStyle;
          try {
            const saved: BadgeStyle = await entitlementsAPI.setBadgeStyle(target);
            confirmedBadgeStyle = saved;
            if (desiredBadgeStyle === target) {
              desiredBadgeStyle = null;
              set({ badgeStyle: saved });
            }
          } catch (err) {
            if (desiredBadgeStyle !== target) continue;
            desiredBadgeStyle = null;
            set({ badgeStyle: confirmedBadgeStyle });
            throw err;
          }
        }
      } finally {
        badgeInFlight = null;
      }
    })();
    return badgeInFlight;
  },

  clear: () => {
    generation += 1;
    desiredBadgeStyle = null;
    confirmedBadgeStyle = null;
    set({ tier: "free", badgeStyle: DEFAULT_BADGE_STYLE, loaded: false });
  },
}));
