import { create } from "zustand";

import { userAPI } from "../api/user";
import { acceptedFriendIds } from "./friendshipStore";
import { useAuthStore } from "../../auth/store/authStore";

export interface GameActivity {
  gameId: string;
  name: string;
  startedAt: string;
}

interface ActivityState {
  byUser: Record<string, GameActivity>;
  apply: (userId: string, activity: GameActivity | null) => void;
  applyEvent: (
    userId: string,
    activity: GameActivity | null,
    occurredAt: string | null,
  ) => void;
  applyBulk: (
    items: (GameActivity & { userId: string; changedAt: string })[],
    requestedIds: string[],
    snapshot: Record<string, number>,
    asOf: string,
  ) => void;
  clearAll: () => void;
}

const REVALIDATE_MS = 60_000;

const seqByUser: Record<string, number> = {};
const lastEventAt: Record<string, number> = {};
let loadSession = 0;
let revalidateTimer: number | null = null;

const bumpSeq = (userId: string) => {
  seqByUser[userId] = (seqByUser[userId] ?? 0) + 1;
};

const raiseEventFloor = (userId: string, ts: number) => {
  if (!Number.isFinite(ts)) return;
  const last = lastEventAt[userId];
  if (last === undefined || ts > last) lastEventAt[userId] = ts;
};

const stopRevalidation = () => {
  if (revalidateTimer === null) return;
  window.clearInterval(revalidateTimer);
  revalidateTimer = null;
};

const ensureRevalidation = () => {
  if (revalidateTimer !== null) return;
  revalidateTimer = window.setInterval(() => {
    const ids = Object.keys(useActivityStore.getState().byUser);
    if (ids.length === 0) {
      stopRevalidation();
      return;
    }
    void loadActivitiesFor(ids);
  }, REVALIDATE_MS);
};

export const useActivityStore = create<ActivityState>()((set) => ({
  byUser: {},

  apply: (userId, activity) =>
    set((state) => {
      bumpSeq(userId);
      if (!activity) {
        if (!state.byUser[userId]) return state;
        const { [userId]: _gone, ...rest } = state.byUser;
        return { byUser: rest };
      }
      ensureRevalidation();
      return { byUser: { ...state.byUser, [userId]: activity } };
    }),

  applyEvent: (userId, activity, occurredAt) => {
    const ts = occurredAt ? Date.parse(occurredAt) : NaN;
    if (Number.isFinite(ts)) {
      const last = lastEventAt[userId];
      if (last !== undefined && ts < last) return;
      lastEventAt[userId] = ts;
    }
    useActivityStore.getState().apply(userId, activity);
  },

  applyBulk: (items, requestedIds, snapshot, asOf) =>
    set((state) => {
      const next = { ...state.byUser };
      const floor = Date.parse(asOf);
      const fresh = new Set(
        requestedIds.filter(
          (id) =>
            (seqByUser[id] ?? 0) === (snapshot[id] ?? 0) &&
            !(Number.isFinite(floor) && (lastEventAt[id] ?? -Infinity) > floor),
        ),
      );
      for (const id of fresh) {
        delete next[id];
        raiseEventFloor(id, floor);
      }
      for (const item of items) {
        if (!fresh.has(item.userId)) continue;
        next[item.userId] = {
          gameId: item.gameId,
          name: item.name,
          startedAt: item.startedAt,
        };
        raiseEventFloor(item.userId, Date.parse(item.changedAt));
        ensureRevalidation();
      }
      return { byUser: next };
    }),

  clearAll: () => {
    stopRevalidation();
    loadSession += 1;
    for (const key of Object.keys(seqByUser)) delete seqByUser[key];
    for (const key of Object.keys(lastEventAt)) delete lastEventAt[key];
    set({ byUser: {} });
  },
}));

const trackedIds = new Map<string, string[]>();

export function trackActivityIds(key: string, ids: string[] | null): void {
  if (ids === null) trackedIds.delete(key);
  else trackedIds.set(key, ids);
}

export function trackedActivityIds(): string[] {
  return Array.from(new Set(Array.from(trackedIds.values()).flat()));
}

export async function loadActivitiesFor(userIds: string[]): Promise<void> {
  const selfId = useAuthStore.getState().user?.id ?? null;
  const ids = Array.from(new Set(userIds)).filter((id) => id !== selfId);
  if (ids.length === 0) return;
  const chunks: string[][] = [];
  for (let i = 0; i < ids.length; i += 100) chunks.push(ids.slice(i, i + 100));
  const session = loadSession;
  await Promise.all(
    chunks.map((chunk) => {
      const snapshot: Record<string, number> = {};
      for (const id of chunk) snapshot[id] = seqByUser[id] ?? 0;
      return userAPI
        .getActivities(chunk)
        .then(({ asOf, activities }) => {
          if (session !== loadSession) return;
          useActivityStore.getState().applyBulk(activities, chunk, snapshot, asOf);
        })
        .catch(() => {});
    }),
  );
}

export async function loadFriendActivities(): Promise<void> {
  await loadActivitiesFor(acceptedFriendIds());
}
