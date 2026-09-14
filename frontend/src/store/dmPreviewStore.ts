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
  setAll: (
    items: (DmPreview & { peerId: string })[],
    replace?: boolean,
  ) => void;
  push: (friendId: string, preview: DmPreview) => void;
  editMessage: (
    messageId: string,
    newContent: string,
    hasAttachments?: boolean,
  ) => void;
  hasMessage: (messageId: string) => boolean;
  clear: () => void;
}

export const useDmPreviewStore = create<DmPreviewState>()((set, get) => ({
  byFriend: {},

  setAll: (items, replace = false) =>
    set((s) => {
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

  clear: () => set({ byFriend: {} }),
}));

let pendingReplace = false;
const runPreviewsRefetch = coalesce(async () => {
  const replace = pendingReplace;
  pendingReplace = false;
  const { dmAPI } = await import("../api/dm");
  const res = await dmAPI.listPreviews();
  useDmPreviewStore.getState().setAll(res.previews, replace);
});

export function refetchDmPreviews(replace = false): void {
  pendingReplace = pendingReplace || replace;
  void runPreviewsRefetch();
}
