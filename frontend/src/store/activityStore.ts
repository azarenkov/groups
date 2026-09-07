import { create } from "zustand";

import { userAPI } from "../api/user";
import { useFriendshipStore } from "./friendshipStore";
import { useAuthStore } from "./authStore";

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
    items: (GameActivity & { userId: string })[],
    requestedIds: string[],
    snapshot: Record<string, number>,
  ) => void;
  clearAll: () => void;
}

const seqByUser: Record<string, number> = {};
const lastEventAt: Record<string, number> = {};
let loadSession = 0;

const bumpSeq = (userId: string) => {
  seqByUser[userId] = (seqByUser[userId] ?? 0) + 1;
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

  applyBulk: (items, requestedIds, snapshot) =>
    set((state) => {
      const next = { ...state.byUser };
      const fresh = new Set(
        requestedIds.filter((id) => (seqByUser[id] ?? 0) === (snapshot[id] ?? 0)),
      );
      for (const id of fresh) delete next[id];
      for (const item of items) {
        if (!fresh.has(item.userId)) continue;
        next[item.userId] = {
          gameId: item.gameId,
          name: item.name,
          startedAt: item.startedAt,
        };
      }
      return { byUser: next };
    }),

  clearAll: () => {
    loadSession += 1;
    for (const key of Object.keys(seqByUser)) delete seqByUser[key];
    for (const key of Object.keys(lastEventAt)) delete lastEventAt[key];
    set({ byUser: {} });
  },
}));

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
        .then((items) => {
          if (session !== loadSession) return;
          useActivityStore.getState().applyBulk(items, chunk, snapshot);
        })
        .catch(() => {});
    }),
  );
}

export async function loadFriendActivities(): Promise<void> {
  await loadActivitiesFor(
    useFriendshipStore
      .getState()
      .friends.filter((f) => f.status === "accepted")
      .map((f) => f.friendId),
  );
}
