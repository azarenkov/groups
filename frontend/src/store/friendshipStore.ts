import { create } from "zustand";

import { friendshipAPI } from "../api/friendship";
import { Friendship } from "../types/friendship";

interface FriendshipState {
  friends: Friendship[];
  isLoading: boolean;
  error: string | null;

  fetchFriends: () => Promise<void>;
  sendRequest: (username: string) => Promise<void>;
  accept: (friendId: string) => Promise<void>;
  remove: (friendId: string) => Promise<void>;
  clearError: () => void;
}

export const useFriendshipStore = create<FriendshipState>((set, get) => ({
  friends: [],
  isLoading: false,
  error: null,

  fetchFriends: async () => {
    set({ isLoading: true, error: null });
    try {
      const res = await friendshipAPI.list();
      set({ friends: res.friends, isLoading: false });
    } catch (e) {
      set({
        error: e instanceof Error ? e.message : "Failed to load friends",
        isLoading: false,
      });
    }
  },

  sendRequest: async (username) => {
    try {
      await friendshipAPI.sendRequest(username);
      await get().fetchFriends();
    } catch (e) {
      throw e;
    }
  },

  accept: async (friendId) => {
    await friendshipAPI.accept(friendId);
    await get().fetchFriends();
  },

  remove: async (friendId) => {
    await friendshipAPI.remove(friendId);
    set((state) => ({
      friends: state.friends.filter((f) => f.friendId !== friendId),
    }));
  },

  clearError: () => set({ error: null }),
}));
