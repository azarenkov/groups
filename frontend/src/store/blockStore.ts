import { create } from "zustand";

import { friendshipAPI } from "../api/friendship";
import type { BlockedUser } from "../types/friendship";

interface BlockState {
  blocked: Record<string, BlockedUser>;
  blockedBy: Record<string, true>;
  loaded: boolean;
  fetch: () => Promise<void>;
  block: (userId: string) => Promise<void>;
  unblock: (userId: string) => Promise<void>;
  applyEvent: (selfId: string, blockerId: string, blockedId: string, active: boolean) => void;
  markBlockedBy: (userId: string, active: boolean) => void;
  clearAll: () => void;
}

const withoutKey = <T>(map: Record<string, T>, key: string): Record<string, T> => {
  const { [key]: _gone, ...rest } = map;
  return rest;
};

let requestGen = 0;

export const useBlockStore = create<BlockState>()((set, get) => ({
  blocked: {},
  blockedBy: {},
  loaded: false,

  fetch: async () => {
    const gen = ++requestGen;
    try {
      const res = await friendshipAPI.listBlocks();
      if (gen !== requestGen) return;
      const blocked: Record<string, BlockedUser> = {};
      for (const b of res.blocks) blocked[b.userId] = b;
      set({ blocked, loaded: true });
    } catch (err) {
      console.warn("[blocks] failed to load blocked users:", err);
    }
  },

  block: async (userId) => {
    await friendshipAPI.block(userId);
    requestGen += 1;
    set((state) => ({
      blocked: {
        ...state.blocked,
        [userId]: state.blocked[userId] ?? {
          userId,
          name: "",
          createdAt: new Date().toISOString(),
        },
      },
    }));
    void get().fetch();
  },

  unblock: async (userId) => {
    await friendshipAPI.unblock(userId);
    requestGen += 1;
    set((state) => ({ blocked: withoutKey(state.blocked, userId) }));
  },

  applyEvent: (selfId, blockerId, blockedId, active) => {
    if (blockerId === selfId) {
      requestGen += 1;
      if (active) {
        set((state) => ({
          blocked: {
            ...state.blocked,
            [blockedId]: state.blocked[blockedId] ?? {
              userId: blockedId,
              name: "",
              createdAt: new Date().toISOString(),
            },
          },
        }));
        void get().fetch();
      } else {
        set((state) => ({ blocked: withoutKey(state.blocked, blockedId) }));
      }
    }
    if (blockedId === selfId) get().markBlockedBy(blockerId, active);
  },

  markBlockedBy: (userId, active) =>
    set((state) => {
      if (active) {
        if (state.blockedBy[userId]) return state;
        return { blockedBy: { ...state.blockedBy, [userId]: true } };
      }
      if (!state.blockedBy[userId]) return state;
      return { blockedBy: withoutKey(state.blockedBy, userId) };
    }),

  clearAll: () => {
    requestGen += 1;
    set({ blocked: {}, blockedBy: {}, loaded: false });
  },
}));

export const isBlockedEitherWay = (userId: string): boolean => {
  const s = useBlockStore.getState();
  return !!s.blocked[userId] || !!s.blockedBy[userId];
};
