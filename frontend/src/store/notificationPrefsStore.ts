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

const channelSeqs: Record<string, number> = {};
const dmSeqs: Record<string, number> = {};
const writeQueues: Record<string, Promise<void>> = {};
let sessionGen = 0;

function enqueueWrite(key: string, task: () => Promise<void>): Promise<void> {
  const next = (writeQueues[key] ?? Promise.resolve()).then(task, task);
  writeQueues[key] = next.catch(() => {});
  return next;
}

export const useNotificationPrefsStore = create<NotificationPrefsState>()(
  (set, get) => ({
    channelLevels: {},
    dmLevels: {},

    load: async () => {
      const gen = sessionGen;
      const sessionUserId = useAuthStore.getState().user?.id;
      if (!sessionUserId) return;
      const { preferences } = await userAPI.getNotificationPreferences();
      if (
        sessionGen !== gen ||
        useAuthStore.getState().user?.id !== sessionUserId
      ) {
        return;
      }
      const channelLevels: Record<string, NotifLevel> = {};
      const dmLevels: Record<string, NotifLevel> = {};
      for (const p of preferences) {
        if (!isLevel(p.level)) continue;
        if (p.scopeType === "channel") channelLevels[p.scopeId] = p.level;
        else if (p.scopeType === "dm") dmLevels[p.scopeId] = p.level;
      }
      set((s) => {
        for (const key of Object.keys(channelSeqs)) {
          if (s.channelLevels[key] === undefined) delete channelLevels[key];
          else channelLevels[key] = s.channelLevels[key];
        }
        for (const key of Object.keys(dmSeqs)) {
          if (s.dmLevels[key] === undefined) delete dmLevels[key];
          else dmLevels[key] = s.dmLevels[key];
        }
        return { channelLevels, dmLevels };
      });
    },

    setChannelLevel: async (channelId, level) => {
      const gen = sessionGen;
      const sessionUserId = useAuthStore.getState().user?.id;
      const seq = (channelSeqs[channelId] = (channelSeqs[channelId] ?? 0) + 1);
      const prev = get().channelLevels[channelId];
      set((s) => ({
        channelLevels: { ...s.channelLevels, [channelId]: level },
      }));
      const current = () =>
        sessionGen === gen &&
        channelSeqs[channelId] === seq &&
        useAuthStore.getState().user?.id === sessionUserId;
      await enqueueWrite(`channel:${channelId}`, async () => {
        if (!current()) return;
        try {
          await userAPI.setNotificationPreference("channel", channelId, level);
          if (current()) {
            void refetchUnread().catch(() => {});
          }
        } catch (e) {
          if (current()) {
            set((s) => {
              const next = { ...s.channelLevels };
              if (prev === undefined) delete next[channelId];
              else next[channelId] = prev;
              return { channelLevels: next };
            });
          }
          throw e;
        }
      });
    },

    setDmLevel: async (friendId, level) => {
      const gen = sessionGen;
      const sessionUserId = useAuthStore.getState().user?.id;
      const seq = (dmSeqs[friendId] = (dmSeqs[friendId] ?? 0) + 1);
      const prev = get().dmLevels[friendId];
      set((s) => ({ dmLevels: { ...s.dmLevels, [friendId]: level } }));
      const current = () =>
        sessionGen === gen &&
        dmSeqs[friendId] === seq &&
        useAuthStore.getState().user?.id === sessionUserId;
      await enqueueWrite(`dm:${friendId}`, async () => {
        if (!current()) return;
        try {
          await userAPI.setNotificationPreference("dm", friendId, level);
          if (current()) {
            void refetchUnread().catch(() => {});
          }
        } catch (e) {
          if (current()) {
            set((s) => {
              const next = { ...s.dmLevels };
              if (prev === undefined) delete next[friendId];
              else next[friendId] = prev;
              return { dmLevels: next };
            });
          }
          throw e;
        }
      });
    },

    clearAll: () => {
      sessionGen += 1;
      for (const k of Object.keys(channelSeqs)) delete channelSeqs[k];
      for (const k of Object.keys(dmSeqs)) delete dmSeqs[k];
      for (const k of Object.keys(writeQueues)) delete writeQueues[k];
      set({ channelLevels: {}, dmLevels: {} });
    },
  }),
);

export function channelNotifLevel(channelId: string): NotifLevel {
  return useNotificationPrefsStore.getState().channelLevels[channelId] ?? "all";
}

export function dmNotifLevel(friendId: string): NotifLevel {
  return useNotificationPrefsStore.getState().dmLevels[friendId] ?? "all";
}
