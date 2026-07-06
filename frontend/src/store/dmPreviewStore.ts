import { create } from "zustand";

export interface DmPreview {
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
  clear: () => void;
}

export const useDmPreviewStore = create<DmPreviewState>()((set) => ({
  byFriend: {},

  setAll: (items) =>
    set(() => {
      const map: Record<string, DmPreview> = {};
      for (const it of items) {
        map[it.peerId] = {
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
      if (cur && new Date(cur.createdAt).getTime() > new Date(preview.createdAt).getTime()) {
        return s;
      }
      return { byFriend: { ...s.byFriend, [friendId]: preview } };
    }),

  clear: () => set({ byFriend: {} }),
}));
