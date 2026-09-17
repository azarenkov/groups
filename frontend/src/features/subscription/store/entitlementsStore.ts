import { create } from "zustand";

import { entitlementsAPI } from "../api/entitlements";
import { DEFAULT_BADGE_STYLE, DEFAULT_STREAM_QUALITY } from "../lib/plans";
import type { BadgeStyle, StreamQuality, Tier } from "../lib/plans";

const STREAM_QUALITY_KEY = "rift:streamQuality";

const readStreamQuality = (): StreamQuality["id"] => {
  try {
    const raw = localStorage.getItem(STREAM_QUALITY_KEY);
    if (raw === "720p60" || raw === "1080p60" || raw === "1440p60" || raw === "2160p60") {
      return raw;
    }
  } catch {}
  return DEFAULT_STREAM_QUALITY;
};

interface EntitlementsState {
  tier: Tier;
  badgeStyle: BadgeStyle;
  loaded: boolean;
  streamQuality: StreamQuality["id"];
  fetch: () => Promise<void>;
  applyTier: (tier: Tier) => void;
  applyBadgeStyle: (style: BadgeStyle) => void;
  setBadgeStyle: (style: BadgeStyle) => Promise<void>;
  setStreamQuality: (quality: StreamQuality["id"]) => void;
  clear: () => void;
}

let generation = 0;

export const useEntitlementsStore = create<EntitlementsState>()((set, get) => ({
  tier: "free",
  badgeStyle: DEFAULT_BADGE_STYLE,
  loaded: false,
  streamQuality: readStreamQuality(),

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

  setStreamQuality: (streamQuality) => {
    set({ streamQuality });
    try {
      localStorage.setItem(STREAM_QUALITY_KEY, streamQuality);
    } catch {}
  },

  clear: () => {
    generation += 1;
    set({ tier: "free", badgeStyle: DEFAULT_BADGE_STYLE, loaded: false });
  },
}));
