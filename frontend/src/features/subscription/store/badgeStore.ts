import { create } from "zustand";

import { entitlementsAPI } from "../api/entitlements";
import { DEFAULT_BADGE_STYLE } from "../lib/plans";
import type { BadgeStyle, Tier } from "../lib/plans";

const BATCH_LIMIT = 200;
const BATCH_DELAY_MS = 40;
const REVALIDATE_MS = 5 * 60_000;
const REVALIDATE_AFTER_EVENT_MS = 5_000;
const RETRY_BASE_MS = 1_000;
const MAX_RETRIES = 3;

export interface Badge {
  tier: Tier;
  badgeStyle: BadgeStyle;
}

interface BadgeState {
  byUser: Record<string, Badge | null>;
  request: (userId: string) => void;
  apply: (userId: string, badge: Badge | null) => void;
  applyStyle: (userId: string, badgeStyle: BadgeStyle) => void;
  applyTier: (userId: string, tier: Tier) => void;
  forget: (userId: string) => void;
  revalidateLater: (userId: string) => void;
  clear: () => void;
}

const queued = new Set<string>();
const fetchedAt = new Map<string, number>();
const retries = new Map<string, number>();
let timer: ReturnType<typeof setTimeout> | null = null;
let generation = 0;

const flush = () => {
  timer = null;
  const ids = Array.from(queued).slice(0, BATCH_LIMIT);
  for (const id of ids) queued.delete(id);
  if (queued.size > 0) timer = setTimeout(flush, BATCH_DELAY_MS);
  if (ids.length === 0) return;
  const gen = generation;
  entitlementsAPI
    .lookupBadges(ids)
    .then((badges) => {
      if (gen !== generation) return;
      const found = new Map(badges.map((b) => [b.userId, b]));
      const now = Date.now();
      useBadgeStore.setState((state) => {
        const byUser = { ...state.byUser };
        for (const id of ids) {
          const badge = found.get(id);
          byUser[id] = badge ? { tier: badge.tier, badgeStyle: badge.badgeStyle } : null;
          fetchedAt.set(id, now);
          retries.delete(id);
        }
        return { byUser };
      });
    })
    .catch((err) => {
      if (gen !== generation) return;
      console.warn("[badges] lookup failed:", err);
      let backoff = 0;
      for (const id of ids) {
        fetchedAt.delete(id);
        const attempt = retries.get(id) ?? 0;
        if (attempt >= MAX_RETRIES) {
          retries.delete(id);
          continue;
        }
        retries.set(id, attempt + 1);
        queued.add(id);
        backoff = Math.max(backoff, RETRY_BASE_MS * 2 ** attempt);
      }
      if (backoff > 0 && !timer) timer = setTimeout(flush, backoff);
    });
};

export const useBadgeStore = create<BadgeState>()((set) => ({
  byUser: {},

  request: (userId) => {
    const seen = fetchedAt.get(userId);
    if (seen !== undefined && Date.now() - seen < REVALIDATE_MS) return;
    fetchedAt.set(userId, Date.now());
    queued.add(userId);
    if (!timer) timer = setTimeout(flush, BATCH_DELAY_MS);
  },

  apply: (userId, badge) => {
    fetchedAt.set(userId, Date.now());
    set((state) => ({ byUser: { ...state.byUser, [userId]: badge } }));
  },

  applyStyle: (userId, badgeStyle) =>
    set((state) => {
      const current = state.byUser[userId];
      if (!current) return state;
      return { byUser: { ...state.byUser, [userId]: { ...current, badgeStyle } } };
    }),

  applyTier: (userId, tier) => {
    fetchedAt.set(userId, Date.now());
    set((state) => {
      if (tier === "free") return { byUser: { ...state.byUser, [userId]: null } };
      const current = state.byUser[userId];
      const badgeStyle = current?.badgeStyle ?? DEFAULT_BADGE_STYLE;
      return { byUser: { ...state.byUser, [userId]: { tier, badgeStyle } } };
    });
  },

  forget: (userId) => {
    fetchedAt.delete(userId);
    queued.delete(userId);
  },

  revalidateLater: (userId) => {
    const gen = generation;
    setTimeout(() => {
      if (gen !== generation) return;
      fetchedAt.delete(userId);
      useBadgeStore.getState().request(userId);
    }, REVALIDATE_AFTER_EVENT_MS);
  },

  clear: () => {
    generation += 1;
    queued.clear();
    fetchedAt.clear();
    retries.clear();
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    set({ byUser: {} });
  },
}));
