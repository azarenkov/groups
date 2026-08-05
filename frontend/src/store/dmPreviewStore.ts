import { create } from "zustand";

export interface DmPreview {
  messageId: string;
  senderId: string;
  content: string;
  callStatus: string | null;
  inviteServerId: string | null;
  createdAt: string;
}

interface DmPreviewState {
  byFriend: Record<string, DmPreview>;
  setAll: (items: (DmPreview & { peerId: string })[]) => void;
  push: (friendId: string, preview: DmPreview) => void;
  editMessage: (messageId: string, newContent: string) => void;
  hasMessage: (messageId: string) => boolean;
  clear: () => void;
}

export const useDmPreviewStore = create<DmPreviewState>()((set, get) => ({
  byFriend: {},

  setAll: (items) =>
    set(() => {
      const map: Record<string, DmPreview> = {};
      for (const it of items) {
        map[it.peerId] = {
          messageId: it.messageId,
          senderId: it.senderId,
          content: it.content,
          callStatus: it.callStatus,
          inviteServerId: it.inviteServerId,
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
      return { byFriend: { ...s.byFriend, [friendId]: preview } };
    }),

  editMessage: (messageId, newContent) =>
    set((s) => {
      const entry = Object.entries(s.byFriend).find(
        ([, p]) => p.messageId === messageId,
      );
      if (!entry) return s;
      const [friendId, p] = entry;
      return {
        byFriend: {
          ...s.byFriend,
          [friendId]: { ...p, content: newContent },
        },
      };
    }),

  hasMessage: (messageId) =>
    Object.values(get().byFriend).some((p) => p.messageId === messageId),

  clear: () => set({ byFriend: {} }),
}));

export function refetchDmPreviews(): void {
  void import("../api/dm").then(({ dmAPI }) =>
    dmAPI
      .listPreviews()
      .then((res) => useDmPreviewStore.getState().setAll(res.previews))
      .catch(() => {}),
  );
}
