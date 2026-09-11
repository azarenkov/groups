import { create } from "zustand";

import { friendshipAPI } from "../api/friendship";
import type { BlockedUser, BlockRelation } from "../types/friendship";

interface BlockState {
  blocked: Record<string, BlockedUser>;
  blockedBy: Record<string, true>;
  loaded: boolean;
  fetch: () => Promise<void>;
  block: (userId: string) => Promise<void>;
  unblock: (userId: string) => Promise<void>;
  applyEvent: (
    selfId: string,
    blockerId: string,
    blockedId: string,
    active: boolean,
    occurredAt?: string,
  ) => boolean;
  applyRelation: (userId: string, relation: BlockRelation, gen?: number) => void;
  markBlockedBy: (userId: string, active: boolean) => void;
  clearAll: () => void;
}

const withoutKey = <T>(map: Record<string, T>, key: string): Record<string, T> => {
  const { [key]: _gone, ...rest } = map;
  return rest;
};

let mutationGen = 0;
let fetchSeq = 0;
let outgoingVersions: Record<string, number> = {};
let incomingVersions: Record<string, number> = {};

const recordVersion = (
  versions: Record<string, number>,
  peerId: string,
  occurredAt?: string,
): boolean => {
  const ts = occurredAt ? Date.parse(occurredAt) : NaN;
  if (!Number.isFinite(ts)) return true;
  if (ts < (versions[peerId] ?? 0)) return false;
  versions[peerId] = ts;
  return true;
};

export const useBlockStore = create<BlockState>()((set, get) => ({
  blocked: {},
  blockedBy: {},
  loaded: false,

  fetch: async () => {
    const gen = mutationGen;
    const seq = ++fetchSeq;
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        const res = await friendshipAPI.listBlocks();
        if (gen !== mutationGen || seq !== fetchSeq) return;
        const blocked: Record<string, BlockedUser> = {};
        for (const b of res.blocks) blocked[b.userId] = b;
        set({ blocked, loaded: true });
        return;
      } catch (err) {
        console.warn("[blocks] failed to load blocked users:", err);
        if (gen !== mutationGen || seq !== fetchSeq) return;
        await new Promise((resolve) => setTimeout(resolve, 1000 * (attempt + 1)));
      }
    }
  },

  block: async (userId) => {
    const { occurredAt } = await friendshipAPI.block(userId);
    if (!recordVersion(outgoingVersions, userId, occurredAt)) return;
    mutationGen += 1;
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
    void import("./friendshipStore").then((m) =>
      m.useFriendshipStore.getState().fetchFriends(),
    );
    void import("./unreadStore").then((m) => m.useUnreadStore.getState().resetDm(userId));
  },

  unblock: async (userId) => {
    const { occurredAt } = await friendshipAPI.unblock(userId);
    if (!recordVersion(outgoingVersions, userId, occurredAt)) return;
    mutationGen += 1;
    set((state) => ({ blocked: withoutKey(state.blocked, userId) }));
  },

  applyEvent: (selfId, blockerId, blockedId, active, occurredAt) => {
    if (blockerId === selfId) {
      if (!recordVersion(outgoingVersions, blockedId, occurredAt)) return false;
    } else if (blockedId === selfId) {
      if (!recordVersion(incomingVersions, blockerId, occurredAt)) return false;
    } else {
      return false;
    }
    mutationGen += 1;
    if (blockerId === selfId) {
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
    return true;
  },

  applyRelation: (userId, relation, gen) => {
    if (gen !== undefined && gen !== mutationGen) return;
    get().markBlockedBy(userId, relation.blockedBy);
    const known = !!get().blocked[userId];
    if (relation.blocked && !known) {
      set((state) => ({
        blocked: {
          ...state.blocked,
          [userId]: { userId, name: "", createdAt: new Date().toISOString() },
        },
      }));
      void get().fetch();
    } else if (!relation.blocked && known) {
      set((state) => ({ blocked: withoutKey(state.blocked, userId) }));
    }
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
    mutationGen += 1;
    outgoingVersions = {};
    incomingVersions = {};
    set({ blocked: {}, blockedBy: {}, loaded: false });
  },
}));

export const blockGeneration = (): number => mutationGen;

export const isBlockedEitherWay = (userId: string): boolean => {
  const s = useBlockStore.getState();
  return !!s.blocked[userId] || !!s.blockedBy[userId];
};
