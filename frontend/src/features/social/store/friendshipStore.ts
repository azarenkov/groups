import { create } from "zustand";

import { friendshipAPI } from "../api/friendship";
import { useAuthStore } from "../../auth/store/authStore";
import { Friendship, FriendshipStatus } from "../../../shared/types/friendship";
import { coalesce } from "../../../shared/lib/coalesce";

const PAGE_SIZE = 100;

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

interface FriendshipState {
  friends: Friendship[];
  statusById: Record<string, FriendshipStatus>;
  nextCursor: string | null;
  loaded: boolean;
  isLoading: boolean;
  isLoadingMore: boolean;
  error: string | null;

  fetchFriends: () => Promise<void>;
  loadMore: () => Promise<void>;
  loadAll: () => Promise<void>;
  applyName: (userId: string, name: string) => void;
  forgetFriend: (userId: string) => void;
  clearAll: () => void;
  sendRequest: (username: string) => Promise<void>;
  accept: (friendId: string) => Promise<void>;
  remove: (friendId: string) => Promise<void>;
  clearError: () => void;
}

let fetchSeq = 0;
let fetchInflight: Promise<void> | null = null;
type PageResult = "ok" | "stale" | "failed" | "done";

const MAX_LOAD_ALL_PAGES = 1000;

let sessionGen = 0;
let removalGen = 0;
const removedAt = new Map<string, number>();

const markRemoved = (friendId: string) => {
  removalGen += 1;
  removedAt.set(friendId, removalGen);
};

const removedSince = (startGen: number) => (friendId: string) =>
  (removedAt.get(friendId) ?? 0) > startGen;

const cursorFor = (
  friends: Friendship[],
  statusById: Record<string, FriendshipStatus>,
  cursor: string | null,
): string | null => {
  if (!cursor) return null;
  const loaded = new Set(friends.map((f) => f.friendId));
  return Object.keys(statusById).some((id) => !loaded.has(id)) ? cursor : null;
};

const pruneRemovals = (startGen: number) => {
  removedAt.forEach((gen, friendId) => {
    if (gen <= startGen) removedAt.delete(friendId);
  });
};
let loadMoreInflight: { promise: Promise<PageResult>; seq: number; userId: string | undefined } | null = null;
let loadAllInflight: { promise: Promise<void>; gen: number; userId: string | undefined } | null = null;

export const useFriendshipStore = create<FriendshipState>((set, get) => {
  const runFetchFriends = coalesce(async () => {
    const seq = ++fetchSeq;
    const sessionUserId = useAuthStore.getState().user?.id;
    const startGen = removalGen;
    set({ isLoading: true, error: null });
    try {
      const [ids, page] = await Promise.all([
        friendshipAPI.listIds(),
        friendshipAPI.list({ limit: PAGE_SIZE }),
      ]);
      if (seq !== fetchSeq) return;
      if (useAuthStore.getState().user?.id !== sessionUserId) return;
      const dropped = removedSince(startGen);
      pruneRemovals(startGen);
      const statusById: Record<string, FriendshipStatus> = {};
      for (const f of ids.friends) {
        if (!dropped(f.friendId)) statusById[f.friendId] = f.status;
      }
      const firstPage = page.friends.filter((f) => statusById[f.friendId] !== undefined);
      set((state) => {
        const fresh = new Set(firstPage.map((f) => f.friendId));
        const tail = state.friends
          .filter((f) => !fresh.has(f.friendId) && statusById[f.friendId] !== undefined)
          .map((f) => ({ ...f, status: statusById[f.friendId] }));
        const friends = [...firstPage, ...tail];
        return {
          friends,
          statusById,
          nextCursor: cursorFor(friends, statusById, page.nextCursor),
          loaded: true,
          isLoading: false,
          isLoadingMore: false,
        };
      });
    } catch (e) {
      if (seq !== fetchSeq) return;
      set({
        error: e instanceof Error ? e.message : "Failed to load friends",
        isLoading: false,
        isLoadingMore: false,
      });
    }
  });
  const loadNextPage = (): Promise<PageResult> => {
    const seq = fetchSeq;
    const sessionUserId = useAuthStore.getState().user?.id;
    if (loadMoreInflight && loadMoreInflight.seq === seq && loadMoreInflight.userId === sessionUserId) {
      return loadMoreInflight.promise;
    }
    const { nextCursor } = get();
    if (!nextCursor) return Promise.resolve("done");
    const startGen = removalGen;
    set({ isLoadingMore: true });
    const promise: Promise<PageResult> = (async (): Promise<PageResult> => {
      try {
        const page = await friendshipAPI.list({ cursor: nextCursor, limit: PAGE_SIZE });
        if (seq !== fetchSeq || useAuthStore.getState().user?.id !== sessionUserId) {
          set({ isLoadingMore: false });
          return "stale";
        }
        const dropped = removedSince(startGen);
        set((state) => {
          const known = new Set(state.friends.map((f) => f.friendId));
          const fresh = page.friends
            .filter(
              (f) =>
                !known.has(f.friendId) &&
                !dropped(f.friendId) &&
                (!state.loaded || state.statusById[f.friendId] !== undefined),
            )
            .map((f) => ({ ...f, status: state.statusById[f.friendId] ?? f.status }));
          const friends = [...state.friends, ...fresh];
          return {
            friends,
            nextCursor: state.loaded ? cursorFor(friends, state.statusById, page.nextCursor) : page.nextCursor,
            isLoadingMore: false,
          };
        });
        return "ok";
      } catch {
        set({ isLoadingMore: false });
        return "failed";
      }
    })().finally(() => {
      if (loadMoreInflight?.promise === promise) loadMoreInflight = null;
    });
    loadMoreInflight = { promise, seq, userId: sessionUserId };
    return promise;
  };
  const fetchFriends = () => {
    const request = runFetchFriends();
    fetchInflight = request;
    void request.finally(() => {
      if (fetchInflight === request) fetchInflight = null;
    });
    return request;
  };
  return {
    friends: [],
    statusById: {},
    nextCursor: null,
    loaded: false,
    isLoading: false,
    isLoadingMore: false,
    error: null,

    fetchFriends,

    loadMore: async () => {
      await loadNextPage();
    },

    loadAll: () => {
      const userId = useAuthStore.getState().user?.id;
      if (loadAllInflight && loadAllInflight.gen === sessionGen && loadAllInflight.userId === userId) {
        return loadAllInflight.promise;
      }
      const gen = sessionGen;
      const alive = () => gen === sessionGen && useAuthStore.getState().user?.id === userId;
      const promise: Promise<void> = (async () => {
        if (fetchInflight) await fetchInflight;
        else if (!get().loaded) await fetchFriends();
        let failures = 0;
        for (let pages = 0; pages < MAX_LOAD_ALL_PAGES && alive(); ) {
          if (!get().nextCursor) return;
          const result = await loadNextPage();
          if (fetchInflight) await fetchInflight;
          if (result === "failed") {
            failures += 1;
            await delay(Math.min(30_000, 1000 * 2 ** Math.min(failures - 1, 5)));
            continue;
          }
          failures = 0;
          if (result === "ok") pages += 1;
        }
      })().finally(() => {
        if (loadAllInflight?.promise === promise) loadAllInflight = null;
      });
      loadAllInflight = { promise, gen, userId };
      return promise;
    },

    applyName: (userId, name) =>
      set((state) =>
        state.friends.some((f) => f.friendId === userId && f.friendName !== name)
          ? { friends: state.friends.map((f) => (f.friendId === userId ? { ...f, friendName: name } : f)) }
          : state,
      ),

    forgetFriend: (userId) =>
      set((state) => {
        if (state.statusById[userId] === undefined && !state.friends.some((f) => f.friendId === userId)) {
          return state;
        }
        markRemoved(userId);
        const { [userId]: _gone, ...statusById } = state.statusById;
        return {
          friends: state.friends.filter((f) => f.friendId !== userId),
          statusById,
        };
      }),

    sendRequest: async (username) => {
      await friendshipAPI.sendRequest(username);
      await get().fetchFriends();
    },

    accept: async (friendId) => {
      await friendshipAPI.accept(friendId);
      await get().fetchFriends();
    },

    remove: async (friendId) => {
      await friendshipAPI.remove(friendId);
      markRemoved(friendId);
      set((state) => {
        const { [friendId]: _gone, ...statusById } = state.statusById;
        return {
          friends: state.friends.filter((f) => f.friendId !== friendId),
          statusById,
        };
      });
    },

    clearError: () => set({ error: null }),

    clearAll: () => {
      fetchSeq += 1;
      sessionGen += 1;
      removedAt.clear();
      loadMoreInflight = null;
      loadAllInflight = null;
      fetchInflight = null;
      set({
        friends: [],
        statusById: {},
        nextCursor: null,
        loaded: false,
        isLoading: false,
        isLoadingMore: false,
        error: null,
      });
    },
  };
});

export const acceptedFriendIds = (): string[] =>
  Object.entries(useFriendshipStore.getState().statusById)
    .filter(([, status]) => status === "accepted")
    .map(([id]) => id);
