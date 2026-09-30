import { create } from "zustand";

import { entitlementsAPI } from "../api/entitlements";
import { DEFAULT_BADGE_STYLE } from "../lib/plans";
import type { BadgeStyle, Tier } from "../lib/plans";

export interface PlanWindow {
  validUntil: string | null;
  trial: boolean;
}

interface EntitlementsState {
  tier: Tier;
  badgeStyle: BadgeStyle;
  memberNumber: number | null;
  validUntil: string | null;
  trial: boolean;
  trialAvailable: boolean;
  planVersion: number;
  loaded: boolean;
  fetch: () => Promise<boolean>;
  applyTier: (tier: Tier, memberNumber?: number | null, plan?: PlanWindow) => void;
  startTrial: () => Promise<void>;
  applyBadgeStyle: (style: BadgeStyle) => void;
  setBadgeStyle: (style: BadgeStyle) => Promise<void>;
  clear: () => void;
}

let generation = 0;
let revision = 0;
let remoteBadgeRevision = 0;
let badgeInFlight: Promise<void> | null = null;
let desiredBadgeStyle: BadgeStyle | null = null;
let confirmedBadgeStyle: BadgeStyle | null = null;

export const useEntitlementsStore = create<EntitlementsState>()((set, get) => ({
  tier: "free",
  badgeStyle: DEFAULT_BADGE_STYLE,
  memberNumber: null,
  validUntil: null,
  trial: false,
  trialAvailable: false,
  planVersion: 0,
  loaded: false,

  fetch: async () => {
    const gen = generation;
    const rev = revision;
    try {
      const mine = await entitlementsAPI.getMine();
      if (gen !== generation) return false;
      if (rev !== revision) {
        set({ loaded: true });
        return false;
      }
      confirmedBadgeStyle = mine.badgeStyle;
      set({
        tier: mine.tier,
        badgeStyle: mine.badgeStyle,
        memberNumber: mine.memberNumber,
        validUntil: mine.validUntil,
        trial: mine.trial,
        trialAvailable: mine.trialAvailable,
        planVersion: get().planVersion + 1,
        loaded: true,
      });
      return true;
    } catch (err) {
      if (gen !== generation) return false;
      console.warn("[entitlements] failed to load:", err);
      set({ loaded: true });
      return false;
    }
  },

  applyTier: (tier, memberNumber, plan) => {
    revision += 1;
    const free = tier === "free";
    set({
      tier,
      memberNumber: memberNumber ?? get().memberNumber,
      validUntil: free ? null : plan ? plan.validUntil : get().validUntil,
      trial: free ? false : plan ? plan.trial : get().trial,
      trialAvailable: plan?.trial || !free ? false : get().trialAvailable,
      planVersion: get().planVersion + 1,
    });
    if (free) void get().fetch();
  },

  startTrial: async () => {
    const gen = generation;
    const started = await entitlementsAPI.startTrial();
    if (gen !== generation) return;
    revision += 1;
    set({
      tier: started.tier,
      validUntil: started.validUntil,
      trial: true,
      trialAvailable: false,
      planVersion: get().planVersion + 1,
    });
  },
  applyBadgeStyle: (badgeStyle) => {
    revision += 1;
    remoteBadgeRevision += 1;
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
          const remoteRev = remoteBadgeRevision;
          try {
            const saved: BadgeStyle = await entitlementsAPI.setBadgeStyle(target);
            if (gen !== generation) return;
            if (remoteRev !== remoteBadgeRevision) {
              if (desiredBadgeStyle === target) desiredBadgeStyle = null;
              continue;
            }
            confirmedBadgeStyle = saved;
            if (desiredBadgeStyle === target) {
              desiredBadgeStyle = null;
              set({ badgeStyle: saved });
            }
          } catch (err) {
            if (gen !== generation) return;
            if (remoteRev !== remoteBadgeRevision) {
              if (desiredBadgeStyle === target) desiredBadgeStyle = null;
              continue;
            }
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
    clearExpiryTimer();
    badgeInFlight = null;
    desiredBadgeStyle = null;
    confirmedBadgeStyle = null;
    set({
      tier: "free",
      badgeStyle: DEFAULT_BADGE_STYLE,
      memberNumber: null,
      validUntil: null,
      trial: false,
      trialAvailable: false,
      planVersion: 0,
      loaded: false,
    });
  },
}));

const EXPIRY_GRACE_MS = 2000;
const MAX_TIMER_MS = 2_000_000_000;
let expiryTimer: ReturnType<typeof setTimeout> | null = null;

function clearExpiryTimer() {
  if (expiryTimer !== null) clearTimeout(expiryTimer);
  expiryTimer = null;
}

const lapseLocally = async () => {
  expiryTimer = null;
  const until = useEntitlementsStore.getState().validUntil;
  if (!until) return;
  const fresh = await useEntitlementsStore.getState().fetch();
  const current = useEntitlementsStore.getState();
  if (!fresh && current.validUntil === until && Date.parse(until) <= Date.now() && current.tier !== "free") {
    current.applyTier("free");
  }
};

function scheduleExpiry(validUntil: string | null) {
  clearExpiryTimer();
  if (!validUntil) return;
  const wait = Date.parse(validUntil) - Date.now() + EXPIRY_GRACE_MS;
  if (wait > MAX_TIMER_MS) {
    expiryTimer = setTimeout(() => scheduleExpiry(validUntil), MAX_TIMER_MS);
    return;
  }
  expiryTimer = setTimeout(() => void lapseLocally(), Math.max(0, wait));
}

useEntitlementsStore.subscribe((state, previous) => {
  if (state.validUntil !== previous.validUntil) scheduleExpiry(state.validUntil);
});
