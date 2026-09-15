import { create } from "zustand";

import { friendshipAPI } from "../api/friendship";
import { useAuthStore } from "./authStore";
import { Friendship, FriendshipStatus } from "../types/friendship";
import { coalesce } from "../utils/coalesce";

const PAGE_SIZE = 100;

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
let loadMoreInflight: { promise: Promise<void>; seq: number; userId: string | undefined } | null = null;

export const useFriendshipStore = create<FriendshipState>((set, get) => {
  const runFetchFriends = coalesce(async () => {
    const seq = ++fetchSeq;
    const sessionUserId = useAuthStore.getState().user?.id;
    set({ isLoading: true, error: null });
    try {
      const [ids, page] = await Promise.all([
        friendshipAPI.listIds(),
        friendshipAPI.list({ limit: PAGE_SIZE }),
      ]);
      if (seq !== fetchSeq) return;
      if (useAuthStore.getState().user?.id !== sessionUserId) return;
      const statusById: Record<string, FriendshipStatus> = {};
      for (const f of ids.friends) statusById[f.friendId] = f.status;
      set((state) => {
        const fresh = new Set(page.friends.map((f) => f.friendId));
        const tail = state.friends
          .filter((f) => !fresh.has(f.friendId) && statusById[f.friendId] !== undefined)
          .map((f) => ({ ...f, status: statusById[f.friendId] }));
        return {
          friends: [...page.friends, ...tail],
          statusById,
          nextCursor: tail.length > 0 ? state.nextCursor : page.nextCursor,
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

    loadMore: () => {
      const seq = fetchSeq;
      const sessionUserId = useAuthStore.getState().user?.id;
      if (loadMoreInflight && loadMoreInflight.seq === seq && loadMoreInflight.userId === sessionUserId) {
        return loadMoreInflight.promise;
      }
      const { nextCursor } = get();
      if (!nextCursor) return Promise.resolve();
      set({ isLoadingMore: true });
      const promise: Promise<void> = (async () => {
        try {
          const page = await friendshipAPI.list({ cursor: nextCursor, limit: PAGE_SIZE });
          if (seq !== fetchSeq || useAuthStore.getState().user?.id !== sessionUserId) {
            set({ isLoadingMore: false });
            return;
          }
          set((state) => {
            const known = new Set(state.friends.map((f) => f.friendId));
            const fresh = page.friends.filter((f) => !known.has(f.friendId));
            return {
              friends: [...state.friends, ...fresh],
              nextCursor: page.nextCursor,
              isLoadingMore: false,
            };
          });
        } catch {
          set({ isLoadingMore: false });
        }
      })().finally(() => {
        if (loadMoreInflight?.promise === promise) loadMoreInflight = null;
      });
      loadMoreInflight = { promise, seq, userId: sessionUserId };
      return promise;
    },

    loadAll: async () => {
      if (fetchInflight) await fetchInflight;
      else if (!get().loaded) await fetchFriends();
      for (let guard = 0; guard < 200; guard += 1) {
        const before = get().nextCursor;
        if (!before) return;
        const seqBefore = fetchSeq;
        await get().loadMore();
        if (fetchInflight) await fetchInflight;
        if (get().nextCursor === before && fetchSeq === seqBefore) return;
      }
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
      loadMoreInflight = null;
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
