import { create } from "zustand";

import { userAPI } from "../api/user";
import { useAuthStore } from "./authStore";
import { refetchUnread } from "./unreadStore";

export type NotifLevel = "all" | "mentions" | "none";

interface NotificationPrefsState {
  channelLevels: Record<string, NotifLevel>;
  dmLevels: Record<string, NotifLevel>;
  load: () => Promise<void>;
  setChannelLevel: (channelId: string, level: NotifLevel) => Promise<void>;
  setDmLevel: (friendId: string, level: Exclude<NotifLevel, "mentions">) => Promise<void>;
  clearAll: () => void;
}

function isLevel(v: string): v is NotifLevel {
  return v === "all" || v === "mentions" || v === "none";
}

export const useNotificationPrefsStore = create<NotificationPrefsState>()(
  (set, get) => ({
    channelLevels: {},
    dmLevels: {},

    load: async () => {
      const sessionUserId = useAuthStore.getState().user?.id;
      if (!sessionUserId) return;
      const { preferences } = await userAPI.getNotificationPreferences();
      if (useAuthStore.getState().user?.id !== sessionUserId) return;
      const channelLevels: Record<string, NotifLevel> = {};
      const dmLevels: Record<string, NotifLevel> = {};
      for (const p of preferences) {
        if (!isLevel(p.level)) continue;
        if (p.scopeType === "channel") channelLevels[p.scopeId] = p.level;
        else if (p.scopeType === "dm") dmLevels[p.scopeId] = p.level;
      }
      set({ channelLevels, dmLevels });
    },

    setChannelLevel: async (channelId, level) => {
      const prev = get().channelLevels[channelId];
      set((s) => ({
        channelLevels: { ...s.channelLevels, [channelId]: level },
      }));
      try {
        await userAPI.setNotificationPreference("channel", channelId, level);
        void refetchUnread().catch(() => {});
      } catch (e) {
        set((s) => {
          const next = { ...s.channelLevels };
          if (prev === undefined) delete next[channelId];
          else next[channelId] = prev;
          return { channelLevels: next };
        });
        throw e;
      }
    },

    setDmLevel: async (friendId, level) => {
      const prev = get().dmLevels[friendId];
      set((s) => ({ dmLevels: { ...s.dmLevels, [friendId]: level } }));
      try {
        await userAPI.setNotificationPreference("dm", friendId, level);
        void refetchUnread().catch(() => {});
      } catch (e) {
        set((s) => {
          const next = { ...s.dmLevels };
          if (prev === undefined) delete next[friendId];
          else next[friendId] = prev;
          return { dmLevels: next };
        });
        throw e;
      }
    },

    clearAll: () => set({ channelLevels: {}, dmLevels: {} }),
  }),
);

export function channelNotifLevel(channelId: string): NotifLevel {
  return useNotificationPrefsStore.getState().channelLevels[channelId] ?? "all";
}

export function dmNotifLevel(friendId: string): NotifLevel {
  return useNotificationPrefsStore.getState().dmLevels[friendId] ?? "all";
}
