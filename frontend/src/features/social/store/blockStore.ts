import { create } from "zustand";

import { friendshipAPI } from "../api/friendship";
import { usePresenceStore } from "./presenceStore";
import { profileVersion } from "./userProfileStore";
import { ApiError } from "../../../shared/lib/apiError";
import type { BlockedUser, BlockRelation } from "../../../shared/types/friendship";

interface BlockState {
  blocked: Record<string, true>;
  blockedBy: Record<string, true>;
  loaded: boolean;
  page: BlockedUser[];
  pageCursor: string | null;
  pageLoaded: boolean;
  pageLoading: boolean;
  pageError: boolean;
  fetch: () => Promise<void>;
  fetchPage: () => Promise<void>;
  loadMorePage: () => Promise<void>;
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

const PAGE_SIZE = 50;

let mutationGen = 0;
let fetchSeq = 0;
let pageSeq = 0;
let peerGens: Record<string, number> = {};

const bumpMutation = () => {
  mutationGen += 1;
};

const invalidatePage = () => {
  pageSeq += 1;
  const store = useBlockStore.getState();
  if (store.pageLoaded || store.pageLoading) void store.fetchPage();
};

const dropFromPage = (page: BlockedUser[], userId: string): BlockedUser[] =>
  page.some((b) => b.userId === userId) ? page.filter((b) => b.userId !== userId) : page;

const bumpPeer = (userId: string) => {
  peerGens[userId] = (peerGens[userId] ?? 0) + 1;
};
let outgoingVersions: Record<string, string> = {};
let incomingVersions: Record<string, string> = {};

const recordVersion = (
  versions: Record<string, string>,
  peerId: string,
  occurredAt?: string,
): boolean => {
  const version = profileVersion(occurredAt);
  if (!version) return true;
  if (version < (versions[peerId] ?? "")) return false;
  versions[peerId] = version;
  return true;
};

const PRESENCE_SNAPSHOT_BATCH = 100;
const PRESENCE_SNAPSHOT_ATTEMPTS = 5;

const resyncPresence = (ids: string[]) => {
  if (ids.length === 0) return;
  void import("../api/user").then(({ userAPI }) => {
    for (let start = 0; start < ids.length; start += PRESENCE_SNAPSHOT_BATCH) {
      const batch = ids.slice(start, start + PRESENCE_SNAPSHOT_BATCH);
      const gens = new Map(batch.map((id) => [id, blockGeneration(id)] as const));
      void (async () => {
        for (let attempt = 0; attempt < PRESENCE_SNAPSHOT_ATTEMPTS; attempt += 1) {
          try {
            const res = await userAPI.presenceSnapshot(batch);
            const presence = usePresenceStore.getState();
            for (const id of res.online) {
              if (gens.get(id) !== blockGeneration(id)) continue;
              if (isBlockedEitherWay(id)) continue;
              presence.setOnline(id);
            }
            return;
          } catch {
            await new Promise((resolve) => setTimeout(resolve, 1000 * 2 ** attempt));
          }
        }
      })();
    }
  });
};

const leaveSharedCall = (userId: string) => {
  void import("../../voice/store/callStore").then((m) => {
    const calls = m.useCallStore.getState();
    const shares =
      !!calls.activeCallId &&
      (!!calls.participants[userId] || calls.ringing.includes(userId) || calls.peerId === userId);
    if (shares) void calls.leave().catch(() => {});
  });
};

const reconcileAfterHydration = (blockedIds: string[], blockedByIds: string[]) => {
  void import("../../dm/store/dmPreviewStore").then((m) => m.refetchDmPreviews());
  void import("../../chat/store/unreadStore").then((m) => m.refetchUnread().catch(() => {}));
  void import("../../voice/store/callStore").then((m) => {
    const calls = m.useCallStore.getState();
    for (const id of [...blockedIds, ...blockedByIds]) {
      if (calls.incoming?.callerId === id) calls.clearIncoming();
      calls.clearJoinable(id);
    }
  });
};

const recoverStatus = (userId: string) => {
  void import("./activityStore").then((m) => m.loadActivitiesFor([userId]));
  void import("../../dm/store/dmPreviewStore").then((m) => m.refetchDmPreviews(true));
  void import("../../chat/store/unreadStore").then((m) => m.refetchUnread().catch(() => {}));
  resyncPresence([userId]);
};

const forgetStatus = (userId: string) => {
  usePresenceStore.getState().setOffline(userId);
  void import("./friendshipStore").then((m) => {
    const store = m.useFriendshipStore.getState();
    if (store.statusById[userId] !== undefined || store.friends.some((f) => f.friendId === userId)) {
      store.forgetFriend(userId);
      void store.fetchFriends();
    }
  });
  void import("../../dm/store/dmPreviewStore").then((m) => m.refetchDmPreviews());
  void import("./activityStore").then((m) => m.useActivityStore.getState().apply(userId, null));
  void import("../../chat/store/unreadStore").then((m) => {
    m.useUnreadStore.getState().resetDm(userId);
    void m.refetchUnread().catch(() => {});
  });
  void import("../../voice/store/callStore").then((m) => {
    const calls = m.useCallStore.getState();
    if (calls.incoming?.callerId === userId) calls.clearIncoming();
    calls.clearJoinable(userId);
  });
};

export const useBlockStore = create<BlockState>()((set, get) => ({
  blocked: {},
  blockedBy: {},
  loaded: false,
  page: [],
  pageCursor: null,
  pageLoaded: false,
  pageLoading: false,
  pageError: false,

  fetch: async () => {
    const gen = mutationGen;
    const seq = ++fetchSeq;
    for (let attempt = 0; ; attempt += 1) {
      try {
        const res = await friendshipAPI.listBlockIds();
        if (gen !== mutationGen || seq !== fetchSeq) return;
        const blocked: Record<string, true> = {};
        for (const id of res.ids) blocked[id] = true;
        const previous = get().blocked;
        const firstLoad = !get().loaded;
        const added = Object.keys(blocked).filter((id) => !previous[id]);
        const removed = Object.keys(previous).filter((id) => !blocked[id]);
        if (added.length > 0 || removed.length > 0) {
          bumpMutation();
          invalidatePage();
        }
        for (const id of [...added, ...removed]) bumpPeer(id);
        set({ blocked, loaded: true });
        for (const id of added) {
          forgetStatus(id);
          leaveSharedCall(id);
        }
        if (firstLoad) reconcileAfterHydration(Object.keys(blocked), Object.keys(get().blockedBy));
        if (removed.length > 0) {
          void import("./activityStore").then((m) => m.loadActivitiesFor(removed));
          void import("../../dm/store/dmPreviewStore").then((m) => m.refetchDmPreviews(true));
          void import("../../chat/store/unreadStore").then((m) => m.refetchUnread().catch(() => {}));
          resyncPresence(removed);
        }
        return;
      } catch (err) {
        console.warn("[blocks] failed to load blocked users:", err);
        if (gen !== mutationGen || seq !== fetchSeq) return;
        const delay = Math.min(30_000, 1000 * 2 ** Math.min(attempt, 5));
        await new Promise((resolve) => setTimeout(resolve, delay));
        if (gen !== mutationGen || seq !== fetchSeq) return;
      }
    }
  },

  fetchPage: async () => {
    const seq = ++pageSeq;
    set({ pageLoading: true, pageError: false });
    try {
      const res = await friendshipAPI.listBlocks({ limit: PAGE_SIZE });
      if (seq !== pageSeq) return;
      set({ page: res.blocks, pageCursor: res.nextCursor, pageLoaded: true, pageLoading: false });
    } catch (err) {
      console.warn("[blocks] failed to load blocked users:", err);
      if (seq === pageSeq) set({ pageLoading: false, pageError: true });
    }
  },

  loadMorePage: async () => {
    const { pageCursor, pageLoading } = get();
    if (!pageCursor || pageLoading) return;
    const seq = pageSeq;
    set({ pageLoading: true });
    try {
      const res = await friendshipAPI.listBlocks({ cursor: pageCursor, limit: PAGE_SIZE });
      if (seq !== pageSeq) return;
      set((state) => {
        const known = new Set(state.page.map((b) => b.userId));
        return {
          page: [...state.page, ...res.blocks.filter((b) => !known.has(b.userId))],
          pageCursor: res.nextCursor,
          pageLoading: false,
        };
      });
    } catch {
      if (seq === pageSeq) set({ pageLoading: false });
    }
  },

  block: async (userId) => {
    const { occurredAt } = await friendshipAPI.block(userId);
    if (!recordVersion(outgoingVersions, userId, occurredAt)) return;
    bumpMutation();
    bumpPeer(userId);
    forgetStatus(userId);
    leaveSharedCall(userId);
    set((state) => ({ blocked: { ...state.blocked, [userId]: true } }));
    invalidatePage();
    void get().fetch();
  },

  unblock: async (userId) => {
    const gen = blockGeneration(userId);
    let occurredAt: string | undefined;
    let reconcile = false;
    try {
      ({ occurredAt } = await friendshipAPI.unblock(userId));
    } catch (err) {
      if (!(err instanceof ApiError) || err.code !== "USER_NOT_BLOCKED") throw err;
      if (gen !== blockGeneration(userId)) return;
      reconcile = true;
    }
    if (occurredAt !== undefined && !recordVersion(outgoingVersions, userId, occurredAt)) return;
    bumpMutation();
    bumpPeer(userId);
    set((state) => ({
      blocked: withoutKey(state.blocked, userId),
      page: dropFromPage(state.page, userId),
    }));
    invalidatePage();
    if (reconcile || !get().loaded) void get().fetch();
    recoverStatus(userId);
  },

  applyEvent: (selfId, blockerId, blockedId, active, occurredAt) => {
    if (blockerId === selfId) {
      if (!recordVersion(outgoingVersions, blockedId, occurredAt)) return false;
    } else if (blockedId === selfId) {
      if (!recordVersion(incomingVersions, blockerId, occurredAt)) return false;
    } else {
      return false;
    }
    bumpMutation();
    bumpPeer(blockerId === selfId ? blockedId : blockerId);
    if (blockerId === selfId) {
      if (active) {
        set((state) => ({ blocked: { ...state.blocked, [blockedId]: true } }));
        invalidatePage();
        void get().fetch();
      } else {
        set((state) => ({
          blocked: withoutKey(state.blocked, blockedId),
          page: dropFromPage(state.page, blockedId),
        }));
        invalidatePage();
      }
    }
    if (blockedId === selfId) get().markBlockedBy(blockerId, active);
    if (!get().loaded) void get().fetch();
    return true;
  },

  applyRelation: (userId, relation, gen) => {
    if (gen !== undefined && gen !== blockGeneration(userId)) return;
    const wasBlockedBy = !!get().blockedBy[userId];
    const known = !!get().blocked[userId];
    const changed = wasBlockedBy !== relation.blockedBy || known !== relation.blocked;
    if (changed) {
      bumpMutation();
      bumpPeer(userId);
    }
    get().markBlockedBy(userId, relation.blockedBy);
    if (relation.blocked && !known) {
      if (!(relation.blockedBy && !wasBlockedBy)) forgetStatus(userId);
      leaveSharedCall(userId);
      set((state) => ({ blocked: { ...state.blocked, [userId]: true } }));
      invalidatePage();
      void get().fetch();
    } else if (!relation.blocked && known) {
      set((state) => ({
        blocked: withoutKey(state.blocked, userId),
        page: dropFromPage(state.page, userId),
      }));
      invalidatePage();
      recoverStatus(userId);
    }
    if (!get().loaded) void get().fetch();
  },

  markBlockedBy: (userId, active) => {
    const wasBlockedBy = !!get().blockedBy[userId];
    if (active && !wasBlockedBy) forgetStatus(userId);
    if (!active && wasBlockedBy) recoverStatus(userId);
    set((state) => {
      if (active) {
        if (state.blockedBy[userId]) return state;
        return { blockedBy: { ...state.blockedBy, [userId]: true } };
      }
      if (!state.blockedBy[userId]) return state;
      return { blockedBy: withoutKey(state.blockedBy, userId) };
    });
  },

  clearAll: () => {
    bumpMutation();
    pageSeq += 1;
    peerGens = {};
    outgoingVersions = {};
    incomingVersions = {};
    set({
      blocked: {},
      blockedBy: {},
      loaded: false,
      page: [],
      pageCursor: null,
      pageLoaded: false,
      pageLoading: false,
      pageError: false,
    });
  },
}));

export const blockGeneration = (userId: string): number => peerGens[userId] ?? 0;

export const isBlockedEitherWay = (userId: string): boolean => {
  const s = useBlockStore.getState();
  return !!s.blocked[userId] || !!s.blockedBy[userId];
};
