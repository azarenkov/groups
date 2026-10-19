import { create } from "zustand";

import { userAPI } from "../api/user";
import type { EmojiStatusChange } from "../api/user";
import { useAuthStore } from "../../auth/store/authStore";
import type { EmojiStatus } from "../lib/emojiStatus";
import { profileVersion } from "./userProfileStore";

interface EmojiStatusState {
  byUser: Record<string, EmojiStatus | null>;
  apply: (userId: string, status: EmojiStatus | null, version: string) => void;
  setMine: (stickerId: string) => Promise<void>;
  clearMine: () => Promise<void>;
  clear: () => void;
}

const seenAt = new Map<string, string>();
let generation = 0;

export const useEmojiStatusStore = create<EmojiStatusState>()((set, get) => {
  const applyMine = async (gen: number, change: Promise<EmojiStatusChange>) => {
    const userId = useAuthStore.getState().user?.id;
    const saved = await change;
    if (gen !== generation || !userId) return;
    get().apply(userId, saved.emojiStatus, profileVersion(saved.updatedAt));
  };

  return {
    byUser: {},

    apply: (userId, status, version) => {
      const seen = seenAt.get(userId);
      if (seen !== undefined && seen > version) return;
      seenAt.set(userId, version);
      set((state) => ({ byUser: { ...state.byUser, [userId]: status } }));
    },

    setMine: (stickerId) => applyMine(generation, userAPI.setEmojiStatus(stickerId)),

    clearMine: () => applyMine(generation, userAPI.clearEmojiStatus()),

    clear: () => {
      generation += 1;
      seenAt.clear();
      set({ byUser: {} });
    },
  };
});
