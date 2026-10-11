import { create } from "zustand";

import { entitlementsAPI } from "../api/entitlements";
import { DEFAULT_BADGE_STYLE } from "../lib/plans";
import type { BadgeStyle, Tier } from "../lib/plans";
import type { EmojiStatus } from "../lib/emojiStatus";
import { profileVersion } from "../../social/store/userProfileStore";

const BATCH_LIMIT = 200;
const BATCH_DELAY_MS = 40;
const REVALIDATE_MS = 5 * 60_000;
const REVALIDATE_AFTER_EVENT_MS = 5_000;
const RETRY_BASE_MS = 1_000;
const MAX_RETRIES = 3;

export interface Badge {
  tier: Tier;
  badgeStyle: BadgeStyle;
  memberNumber: number | null;
  emojiStatus?: EmojiStatus | null;
}

interface BadgeState {
  byUser: Record<string, Badge | null>;
  request: (userId: string) => void;
  apply: (userId: string, badge: Badge | null) => void;
  applyStyle: (userId: string, badgeStyle: BadgeStyle) => void;
  applyEmojiStatus: (userId: string, emojiStatus: EmojiStatus | null, version: string) => void;
  applyTier: (userId: string, tier: Tier, memberNumber: number | null) => void;
  forget: (userId: string) => void;
  revalidateLater: (userId: string) => void;
  clear: () => void;
}

const queued = new Set<string>();
const fetchedAt = new Map<string, number>();
const retries = new Map<string, number>();
const statusSeenAt = new Map<string, string>();
const statusFromEvents = new Map<string, EmojiStatus | null>();
let timer: ReturnType<typeof setTimeout> | null = null;
let generation = 0;

const STORAGE_KEY = "rift.badges.v1";
const STORED_LIMIT = 500;

const readStoredBadges = (): Record<string, Badge | null> => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    return parsed && typeof parsed === "object" ? (parsed as Record<string, Badge | null>) : {};
  } catch {
    return {};
  }
};

let saveTimer: ReturnType<typeof setTimeout> | null = null;
const saveBadges = (byUser: Record<string, Badge | null>) => {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = null;
    try {
      const entries = Object.entries(byUser).slice(-STORED_LIMIT);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(Object.fromEntries(entries)));
    } catch {
      return;
    }
  }, 500);
};

const freshestStatus = (userId: string, fetched: EmojiStatus | null, version: string): EmojiStatus | null => {
  const seen = statusSeenAt.get(userId);
  if (seen !== undefined && seen > version) return statusFromEvents.get(userId) ?? null;
  statusSeenAt.set(userId, version);
  statusFromEvents.set(userId, fetched);
  return fetched;
};

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
          byUser[id] = badge
            ? {
                tier: badge.tier,
                badgeStyle: badge.badgeStyle,
                memberNumber: badge.memberNumber,
                emojiStatus: freshestStatus(id, badge.emojiStatus, profileVersion(badge.emojiStatusUpdatedAt)),
              }
            : null;
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
  byUser: readStoredBadges(),

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

  applyEmojiStatus: (userId, emojiStatus, version) => {
    if ((statusSeenAt.get(userId) ?? "") > version) return;
    statusSeenAt.set(userId, version);
    statusFromEvents.set(userId, emojiStatus);
    set((state) => {
      const current = state.byUser[userId];
      if (!current) return state;
      return { byUser: { ...state.byUser, [userId]: { ...current, emojiStatus } } };
    });
  },

  applyTier: (userId, tier, memberNumber) => {
    fetchedAt.set(userId, Date.now());
    set((state) => {
      if (tier === "free") return { byUser: { ...state.byUser, [userId]: null } };
      const current = state.byUser[userId];
      const badgeStyle = current?.badgeStyle ?? DEFAULT_BADGE_STYLE;
      const number = memberNumber ?? current?.memberNumber ?? null;
      return { byUser: { ...state.byUser, [userId]: { ...current, tier, badgeStyle, memberNumber: number } } };
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
    statusSeenAt.clear();
    statusFromEvents.clear();
    if (timer) {
      clearTimeout(timer);
      timer = null;
    }
    set({ byUser: readStoredBadges() });
  },
}));

useBadgeStore.subscribe((state, prev) => {
  if (state.byUser !== prev.byUser && Object.keys(state.byUser).length > 0) saveBadges(state.byUser);
});
