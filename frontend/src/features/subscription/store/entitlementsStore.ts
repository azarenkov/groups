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
let revision = 0;
let badgeInFlight: Promise<void> | null = null;
let desiredBadgeStyle: BadgeStyle | null = null;
let confirmedBadgeStyle: BadgeStyle | null = null;

export const useEntitlementsStore = create<EntitlementsState>()((set, get) => ({
  tier: "free",
  badgeStyle: DEFAULT_BADGE_STYLE,
  loaded: false,

  fetch: async () => {
    const gen = generation;
    const rev = revision;
    try {
      const mine = await entitlementsAPI.getMine();
      if (gen !== generation) return;
      if (rev !== revision) {
        set({ loaded: true });
        return;
      }
      confirmedBadgeStyle = mine.badgeStyle;
      set({ tier: mine.tier, badgeStyle: mine.badgeStyle, loaded: true });
    } catch (err) {
      if (gen !== generation) return;
      console.warn("[entitlements] failed to load:", err);
      set({ loaded: true });
    }
  },

  applyTier: (tier) => {
    revision += 1;
    set({ tier });
  },
  applyBadgeStyle: (badgeStyle) => {
    revision += 1;
    confirmedBadgeStyle = badgeStyle;
    set({ badgeStyle });
  },

  setBadgeStyle: (style) => {
    revision += 1;
    if (confirmedBadgeStyle === null) confirmedBadgeStyle = get().badgeStyle;
    set({ badgeStyle: style });
    desiredBadgeStyle = style;
    if (badgeInFlight) return badgeInFlight;
    const gen = generation;
    badgeInFlight = (async () => {
      try {
        while (gen === generation && desiredBadgeStyle !== null && desiredBadgeStyle !== confirmedBadgeStyle) {
          const target: BadgeStyle = desiredBadgeStyle;
          try {
            const saved: BadgeStyle = await entitlementsAPI.setBadgeStyle(target);
            if (gen !== generation) return;
            confirmedBadgeStyle = saved;
            if (desiredBadgeStyle === target) {
              desiredBadgeStyle = null;
              set({ badgeStyle: saved });
            }
          } catch (err) {
            if (gen !== generation) return;
            if (desiredBadgeStyle !== target) continue;
            desiredBadgeStyle = null;
            set({ badgeStyle: confirmedBadgeStyle });
            throw err;
          }
        }
      } finally {
        if (gen === generation) badgeInFlight = null;
      }
    })();
    return badgeInFlight;
  },

  clear: () => {
    generation += 1;
    badgeInFlight = null;
    desiredBadgeStyle = null;
    confirmedBadgeStyle = null;
    set({ tier: "free", badgeStyle: DEFAULT_BADGE_STYLE, loaded: false });
  },
}));
