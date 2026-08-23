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
  applyRemote: (scopeType: string, scopeId: string, level: string) => void;
  clearAll: () => void;
}

function isLevel(v: string): v is NotifLevel {
  return v === "all" || v === "mentions" || v === "none";
}

const channelSeqs: Record<string, number> = {};
const dmSeqs: Record<string, number> = {};
const channelPersisted: Record<string, NotifLevel> = {};
const dmPersisted: Record<string, NotifLevel> = {};
const writeQueues: Record<string, Promise<void>> = {};
const pendingWrites: Record<string, number> = {};
const deferredRemote: Record<string, boolean> = {};
const writtenScopes = new Set<string>();
const remoteTouched = new Set<string>();
let sessionGen = 0;

function enqueueWrite(key: string, task: () => Promise<void>): Promise<void> {
  const next = (writeQueues[key] ?? Promise.resolve()).then(task, task);
  writeQueues[key] = next.catch(() => {});
  return next;
}

function settleWrite(key: string, scopeType: "channel" | "dm", scopeId: string) {
  pendingWrites[key] = Math.max(0, (pendingWrites[key] ?? 1) - 1);
  if (pendingWrites[key] === 0 && deferredRemote[key]) {
    delete deferredRemote[key];
    void reconcileScope(scopeType, scopeId);
  }
}

async function reconcileScope(
  scopeType: "channel" | "dm",
  scopeId: string,
): Promise<void> {
  const gen = sessionGen;
  const sessionUserId = useAuthStore.getState().user?.id;
  if (!sessionUserId) return;
  try {
    const { preferences } = await userAPI.getNotificationPreferences();
    if (
      sessionGen !== gen ||
      useAuthStore.getState().user?.id !== sessionUserId
    ) {
      return;
    }
    const key = `${scopeType}:${scopeId}`;
    if ((pendingWrites[key] ?? 0) > 0) {
      deferredRemote[key] = true;
      return;
    }
    const found = preferences.find(
      (p) => p.scopeType === scopeType && p.scopeId === scopeId,
    );
    const level =
      found && isLevel(found.level) ? found.level : undefined;
    remoteTouched.add(key);
    if (scopeType === "channel") {
      if (level === undefined) delete channelPersisted[scopeId];
      else channelPersisted[scopeId] = level;
      useNotificationPrefsStore.setState((s) => {
        const next = { ...s.channelLevels };
        if (level === undefined) delete next[scopeId];
        else next[scopeId] = level;
        return { channelLevels: next };
      });
    } else {
      if (level === undefined) delete dmPersisted[scopeId];
      else dmPersisted[scopeId] = level;
      useNotificationPrefsStore.setState((s) => {
        const next = { ...s.dmLevels };
        if (level === undefined) delete next[scopeId];
        else next[scopeId] = level;
        return { dmLevels: next };
      });
    }
    void refetchUnread().catch(() => {});
  } catch {
    return;
  }
}

export const useNotificationPrefsStore = create<NotificationPrefsState>()(
  (set) => ({
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
      const preserveChannel = new Set(Object.keys(channelSeqs));
      const preserveDm = new Set(Object.keys(dmSeqs));
      for (const key of remoteTouched) {
        if (key.startsWith("channel:")) {
          preserveChannel.add(key.slice("channel:".length));
        } else if (key.startsWith("dm:")) {
          preserveDm.add(key.slice("dm:".length));
        }
      }
      for (const [k, v] of Object.entries(channelLevels)) {
        if (!writtenScopes.has(`channel:${k}`) && !remoteTouched.has(`channel:${k}`)) {
          channelPersisted[k] = v;
        }
      }
      for (const [k, v] of Object.entries(dmLevels)) {
        if (!writtenScopes.has(`dm:${k}`) && !remoteTouched.has(`dm:${k}`)) {
          dmPersisted[k] = v;
        }
      }
      set((s) => {
        for (const key of preserveChannel) {
          if (s.channelLevels[key] === undefined) delete channelLevels[key];
          else channelLevels[key] = s.channelLevels[key];
        }
        for (const key of preserveDm) {
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
      set((s) => ({
        channelLevels: { ...s.channelLevels, [channelId]: level },
      }));
      const current = () =>
        sessionGen === gen &&
        channelSeqs[channelId] === seq &&
        useAuthStore.getState().user?.id === sessionUserId;
      await enqueueWrite(`channel:${channelId}`, async () => {
        if (!current()) return;
        const key = `channel:${channelId}`;
        pendingWrites[key] = (pendingWrites[key] ?? 0) + 1;
        try {
          await userAPI.setNotificationPreference("channel", channelId, level);
          if (sessionGen === gen) {
            channelPersisted[channelId] = level;
            writtenScopes.add(key);
          }
          if (current()) {
            void refetchUnread().catch(() => {});
          }
        } catch (e) {
          if (current()) {
            const persisted = channelPersisted[channelId];
            set((s) => {
              const next = { ...s.channelLevels };
              if (persisted === undefined) delete next[channelId];
              else next[channelId] = persisted;
              return { channelLevels: next };
            });
            void refetchUnread().catch(() => {});
          }
          throw e;
        } finally {
          settleWrite(key, "channel", channelId);
        }
      });
    },

    setDmLevel: async (friendId, level) => {
      const gen = sessionGen;
      const sessionUserId = useAuthStore.getState().user?.id;
      const seq = (dmSeqs[friendId] = (dmSeqs[friendId] ?? 0) + 1);
      set((s) => ({ dmLevels: { ...s.dmLevels, [friendId]: level } }));
      const current = () =>
        sessionGen === gen &&
        dmSeqs[friendId] === seq &&
        useAuthStore.getState().user?.id === sessionUserId;
      await enqueueWrite(`dm:${friendId}`, async () => {
        if (!current()) return;
        const key = `dm:${friendId}`;
        pendingWrites[key] = (pendingWrites[key] ?? 0) + 1;
        try {
          await userAPI.setNotificationPreference("dm", friendId, level);
          if (sessionGen === gen) {
            dmPersisted[friendId] = level;
            writtenScopes.add(key);
          }
          if (current()) {
            void refetchUnread().catch(() => {});
          }
        } catch (e) {
          if (current()) {
            const persisted = dmPersisted[friendId];
            set((s) => {
              const next = { ...s.dmLevels };
              if (persisted === undefined) delete next[friendId];
              else next[friendId] = persisted;
              return { dmLevels: next };
            });
            void refetchUnread().catch(() => {});
          }
          throw e;
        } finally {
          settleWrite(key, "dm", friendId);
        }
      });
    },

    applyRemote: (scopeType, scopeId, level) => {
      if (!isLevel(level)) return;
      if (scopeType !== "channel" && scopeType !== "dm") return;
      if ((pendingWrites[`${scopeType}:${scopeId}`] ?? 0) > 0) {
        deferredRemote[`${scopeType}:${scopeId}`] = true;
        return;
      }
      remoteTouched.add(`${scopeType}:${scopeId}`);
      if (scopeType === "channel") {
        channelPersisted[scopeId] = level;
        set((s) => ({
          channelLevels: { ...s.channelLevels, [scopeId]: level },
        }));
      } else {
        dmPersisted[scopeId] = level;
        set((s) => ({ dmLevels: { ...s.dmLevels, [scopeId]: level } }));
      }
    },

    clearAll: () => {
      sessionGen += 1;
      for (const k of Object.keys(channelSeqs)) delete channelSeqs[k];
      for (const k of Object.keys(dmSeqs)) delete dmSeqs[k];
      for (const k of Object.keys(channelPersisted)) delete channelPersisted[k];
      for (const k of Object.keys(dmPersisted)) delete dmPersisted[k];
      for (const k of Object.keys(writeQueues)) delete writeQueues[k];
      for (const k of Object.keys(pendingWrites)) delete pendingWrites[k];
      for (const k of Object.keys(deferredRemote)) delete deferredRemote[k];
      writtenScopes.clear();
      remoteTouched.clear();
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
