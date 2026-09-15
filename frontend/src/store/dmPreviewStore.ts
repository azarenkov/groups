import { create } from "zustand";

import { coalesce } from "../utils/coalesce";

export interface DmPreview {
  messageId: string;
  peerName?: string;
  senderId: string;
  content: string;
  callStatus: string | null;
  inviteServerId: string | null;
  hasAttachments: boolean;
  createdAt: string;
}

interface DmPreviewState {
  byFriend: Record<string, DmPreview>;
  nextCursor: string | null;
  isLoadingMore: boolean;
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
  clear: () => void;
}

let previewGen = 0;
let sessionGen = 0;
let loadMoreToken = 0;
let pendingReplace = false;

export const useDmPreviewStore = create<DmPreviewState>()((set, get) => ({
  byFriend: {},
  nextCursor: null,
  isLoadingMore: false,

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
          inviteServerId: it.inviteServerId,
          hasAttachments: it.hasAttachments,
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

  hasMessage: (messageId) =>
    Object.values(get().byFriend).some((p) => p.messageId === messageId),

  clear: () => {
    previewGen += 1;
    sessionGen += 1;
    pendingReplace = false;
    set({ byFriend: {}, nextCursor: null, isLoadingMore: false });
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
  store.setAll(res.previews, replace);
  if (replace || store.nextCursor === null) store.setNextCursor(res.nextCursor);
});

export function refetchDmPreviews(replace = false): void {
  pendingReplace = pendingReplace || replace;
  void runPreviewsRefetch();
}
