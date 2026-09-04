import { create } from "zustand";

import { userAPI } from "../api/user";
import { useFriendshipStore } from "./friendshipStore";

export interface GameActivity {
  gameId: string;
  name: string;
  startedAt: string;
}

interface ActivityState {
  byUser: Record<string, GameActivity>;
  apply: (userId: string, activity: GameActivity | null) => void;
  applyBulk: (items: (GameActivity & { userId: string })[]) => void;
  clearAll: () => void;
}

export const useActivityStore = create<ActivityState>()((set) => ({
  byUser: {},

  apply: (userId, activity) =>
    set((state) => {
      if (!activity) {
        if (!state.byUser[userId]) return state;
        const { [userId]: _gone, ...rest } = state.byUser;
        return { byUser: rest };
      }
      return { byUser: { ...state.byUser, [userId]: activity } };
    }),

  applyBulk: (items) =>
    set((state) => {
      const next = { ...state.byUser };
      for (const item of items) {
        next[item.userId] = {
          gameId: item.gameId,
          name: item.name,
          startedAt: item.startedAt,
        };
      }
      return { byUser: next };
    }),

  clearAll: () => set({ byUser: {} }),
}));

export async function loadActivitiesFor(userIds: string[]): Promise<void> {
  const ids = Array.from(new Set(userIds));
  if (ids.length === 0) return;
  const chunks: string[][] = [];
  for (let i = 0; i < ids.length; i += 100) chunks.push(ids.slice(i, i + 100));
  const results = await Promise.all(
    chunks.map((chunk) => userAPI.getActivities(chunk).catch(() => [])),
  );
  useActivityStore.getState().applyBulk(results.flat());
}

export async function loadFriendActivities(): Promise<void> {
  await loadActivitiesFor(
    useFriendshipStore
      .getState()
      .friends.filter((f) => f.status === "accepted")
      .map((f) => f.friendId),
  );
}
