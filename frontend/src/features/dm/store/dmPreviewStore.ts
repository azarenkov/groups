import { create } from "zustand";

import { coalesce } from "../../../shared/lib/coalesce";
import { clearedBefore } from "../lib/dmCleared";

export interface DmPreview {
  messageId: string;
  peerName?: string;
  senderId: string;
  content: string;
  callStatus: string | null;
  hasAttachments: boolean;
  voiceDurationMs?: number | null;
  videoNoteDurationMs?: number | null;
  stickerEmoji?: string | null;
  createdAt: string;
}

interface DmPreviewState {
  byFriend: Record<string, DmPreview>;
  nextCursor: string | null;
  isLoadingMore: boolean;
  loaded: boolean;
  setAll: (
    items: (DmPreview & { peerId: string })[],
    replace?: boolean,
  ) => void;
  setNextCursor: (cursor: string | null) => void;
  loadMore: () => Promise<void>;
  push: (friendId: string, preview: DmPreview) => void;
  editMessage: (
    messageId: string,
    newContent: string,
    hasAttachments?: boolean,
  ) => void;
  hasMessage: (messageId: string) => boolean;
  dropCleared: (friendId: string, clearedAt: string) => void;
  clear: () => void;
  hydrate: (items: (DmPreview & { peerId: string })[]) => void;
}

let previewGen = 0;
let sessionGen = 0;
let loadMoreToken = 0;
let pendingReplace = false;
let pagesLoaded = 0;

export const useDmPreviewStore = create<DmPreviewState>()((set, get) => ({
  byFriend: {},
  nextCursor: null,
  isLoadingMore: false,
  loaded: false,

  setNextCursor: (cursor) => set({ nextCursor: cursor }),

  loadMore: async () => {
    const { nextCursor, isLoadingMore } = get();
    if (!nextCursor || isLoadingMore) return;
    const gen = previewGen;
    const token = ++loadMoreToken;
    const release = () => {
      if (token === loadMoreToken) set({ isLoadingMore: false });
    };
    set({ isLoadingMore: true });
    try {
      const { dmAPI } = await import("../api/dm");
      const res = await dmAPI.listPreviews({ cursor: nextCursor });
      if (gen !== previewGen || token !== loadMoreToken) {
        release();
        return;
      }
      get().setAll(res.previews);
      pagesLoaded += 1;
      set({ nextCursor: res.nextCursor, isLoadingMore: false });
    } catch {
      release();
    }
  },

  setAll: (items, replace = false) =>
    set((s) => {
      if (replace) previewGen += 1;
      const map: Record<string, DmPreview> = replace ? {} : { ...s.byFriend };
      for (const it of items) {
        const cur = map[it.peerId];
        if (
          cur &&
          new Date(cur.createdAt).getTime() > new Date(it.createdAt).getTime()
        ) {
          if (it.peerName && !cur.peerName) map[it.peerId] = { ...cur, peerName: it.peerName };
          continue;
        }
        map[it.peerId] = {
          messageId: it.messageId,
          peerName: it.peerName ?? cur?.peerName,
          senderId: it.senderId,
          content: it.content,
          callStatus: it.callStatus,
          hasAttachments: it.hasAttachments,
          voiceDurationMs: it.voiceDurationMs ?? null,
          videoNoteDurationMs: it.videoNoteDurationMs ?? null,
          stickerEmoji: it.stickerEmoji ?? null,
          createdAt: it.createdAt,
        };
      }
      return { byFriend: map };
    }),

  push: (friendId, preview) =>
    set((s) => {
      const cur = s.byFriend[friendId];
      if (
        cur &&
        new Date(cur.createdAt).getTime() > new Date(preview.createdAt).getTime()
      ) {
        return s;
      }
      return { byFriend: { ...s.byFriend, [friendId]: { ...preview, peerName: preview.peerName ?? cur?.peerName } } };
    }),

  editMessage: (messageId, newContent, hasAttachments) =>
    set((s) => {
      const entry = Object.entries(s.byFriend).find(
        ([, p]) => p.messageId === messageId,
      );
      if (!entry) return s;
      const [friendId, p] = entry;
      return {
        byFriend: {
          ...s.byFriend,
          [friendId]: {
            ...p,
            content: newContent,
            hasAttachments: hasAttachments ?? p.hasAttachments,
          },
        },
      };
    }),

  dropCleared: (friendId, clearedAt) =>
    set((s) => {
      const cur = s.byFriend[friendId];
      if (!cur || !clearedBefore(cur.createdAt, clearedAt)) return s;
      const byFriend = { ...s.byFriend };
      delete byFriend[friendId];
      return { byFriend };
    }),

  hasMessage: (messageId) =>
    Object.values(get().byFriend).some((p) => p.messageId === messageId),

  hydrate: (items) => {
    if (get().loaded || Object.keys(get().byFriend).length > 0) return;
    const byFriend: Record<string, DmPreview> = {};
    for (const { peerId, ...preview } of items) byFriend[peerId] = preview;
    pendingReplace = true;
    set({ byFriend });
  },

  clear: () => {
    previewGen += 1;
    sessionGen += 1;
    pendingReplace = false;
    pagesLoaded = 0;
    set({ byFriend: {}, nextCursor: null, isLoadingMore: false, loaded: false });
  },
}));

const runPreviewsRefetch = coalesce(async () => {
  const replace = pendingReplace;
  pendingReplace = false;
  const session = sessionGen;
  const { dmAPI } = await import("../api/dm");
  const res = await dmAPI.listPreviews();
  if (session !== sessionGen) return;
  const store = useDmPreviewStore.getState();
  const rebuildPages = replace ? pagesLoaded : 0;
  store.setAll(res.previews, replace);
  if (replace || store.nextCursor === null) {
    store.setNextCursor(res.nextCursor);
    pagesLoaded = replace ? 1 : Math.max(pagesLoaded, 1);
  }
  if (!useDmPreviewStore.getState().loaded) useDmPreviewStore.setState({ loaded: true });
  const gen = previewGen;
  for (let page = 1; page < rebuildPages; page += 1) {
    const before = useDmPreviewStore.getState().nextCursor;
    if (!before || session !== sessionGen || gen !== previewGen) return;
    await useDmPreviewStore.getState().loadMore();
    if (useDmPreviewStore.getState().nextCursor === before) return;
  }
});

export function refetchDmPreviews(replace = false): void {
  pendingReplace = pendingReplace || replace;
  void runPreviewsRefetch();
}
