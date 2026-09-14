import { create } from "zustand";

import { friendshipAPI } from "../api/friendship";
import { Friendship, FriendshipStatus } from "../types/friendship";
import { coalesce } from "../utils/coalesce";

const PAGE_SIZE = 100;

interface FriendshipState {
  friends: Friendship[];
  statusById: Record<string, FriendshipStatus>;
  nextCursor: string | null;
  isLoading: boolean;
  isLoadingMore: boolean;
  error: string | null;

  fetchFriends: () => Promise<void>;
  loadMore: () => Promise<void>;
  sendRequest: (username: string) => Promise<void>;
  accept: (friendId: string) => Promise<void>;
  remove: (friendId: string) => Promise<void>;
  clearError: () => void;
}

let fetchSeq = 0;

export const useFriendshipStore = create<FriendshipState>((set, get) => {
  const fetchFriends = coalesce(async () => {
    const seq = ++fetchSeq;
    set({ isLoading: true, error: null });
    try {
      const [ids, page] = await Promise.all([
        friendshipAPI.listIds(),
        friendshipAPI.list({ limit: PAGE_SIZE }),
      ]);
      if (seq !== fetchSeq) return;
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
  return {
    friends: [],
    statusById: {},
    nextCursor: null,
    isLoading: false,
    isLoadingMore: false,
    error: null,

    fetchFriends,

    loadMore: async () => {
      const { nextCursor, isLoadingMore } = get();
      if (!nextCursor || isLoadingMore) return;
      const seq = fetchSeq;
      set({ isLoadingMore: true });
      try {
        const page = await friendshipAPI.list({ cursor: nextCursor, limit: PAGE_SIZE });
        if (seq !== fetchSeq) {
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
    },

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
  };
});

export const acceptedFriendIds = (): string[] =>
  Object.entries(useFriendshipStore.getState().statusById)
    .filter(([, status]) => status === "accepted")
    .map(([id]) => id);
