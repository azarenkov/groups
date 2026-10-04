import { create } from "zustand";

import { userAPI } from "../../social/api/user";
import { useAuthStore } from "../../auth/store/authStore";
import { refetchUnread } from "../../chat/store/unreadStore";

export type NotifLevel = "all" | "mentions" | "none";

interface NotificationPrefsState {
  channelLevels: Record<string, NotifLevel>;
  dmLevels: Record<string, NotifLevel>;
  load: () => Promise<void>;
  setChannelLevel: (channelId: string, level: NotifLevel) => Promise<void>;
  setDmLevel: (friendId: string, level: Exclude<NotifLevel, "mentions">) => Promise<void>;
  applyRemote: (
    scopeType: string,
    scopeId: string,
    level: string,
    version?: number,
  ) => void;
  clearAll: () => void;
}

export const dmLevelOf = (
  dmLevels: Record<string, NotifLevel>,
  friendId: string,
  globalMentions: string,
): NotifLevel => dmLevels[friendId] ?? (globalMentions === "none" ? "none" : "all");

function isLevel(v: string): v is NotifLevel {
  return v === "all" || v === "mentions" || v === "none";
}

const channelSeqs: Record<string, number> = {};
const dmSeqs: Record<string, number> = {};
const channelPersisted: Record<string, NotifLevel> = {};
const dmPersisted: Record<string, NotifLevel> = {};
const writeQueues: Record<string, Promise<void>> = {};
const activeWrites: Record<string, number> = {};
const deferredRemote: Record<string, boolean> = {};
const remoteVersions: Record<string, number> = {};
let sessionGen = 0;
let prefsLoaded = false;
let loadGen = 0;

export function notificationPrefsLoaded(): boolean {
  return prefsLoaded;
}

export function markNotificationPrefsStale(): void {
  prefsLoaded = false;
  loadGen += 1;
}

function advanceRemoteVersion(key: string, version: number | undefined): void {
  if (typeof version !== "number" || !Number.isFinite(version)) return;
  remoteVersions[key] = Math.max(remoteVersions[key] ?? -Infinity, version);
}

function isStaleVersion(key: string, version: number | undefined): boolean {
  return (
    typeof version === "number" &&
    remoteVersions[key] !== undefined &&
    version < remoteVersions[key]
  );
}

function enqueueWrite(key: string, task: () => Promise<void>): Promise<void> {
  const next = (writeQueues[key] ?? Promise.resolve()).then(task, task);
  writeQueues[key] = next.catch(() => {});
  return next;
}

function releaseWrite(key: string, scopeType: "channel" | "dm", scopeId: string) {
  activeWrites[key] = Math.max(0, (activeWrites[key] ?? 1) - 1);
  if (activeWrites[key] === 0 && deferredRemote[key]) {
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
  const key = `${scopeType}:${scopeId}`;
  try {
    const { preferences } = await userAPI.getNotificationPreferences();
    if (
      sessionGen !== gen ||
      useAuthStore.getState().user?.id !== sessionUserId
    ) {
      return;
    }
    if ((activeWrites[key] ?? 0) > 0) {
      deferredRemote[key] = true;
      return;
    }
    const found = preferences.find(
      (p) => p.scopeType === scopeType && p.scopeId === scopeId,
    );
    const level =
      found && isLevel(found.level) ? found.level : undefined;
    advanceRemoteVersion(key, found?.version);
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
    if (sessionGen !== gen) return;
    deferredRemote[key] = true;
    setTimeout(() => {
      if (deferredRemote[key] && (activeWrites[key] ?? 0) === 0) {
        delete deferredRemote[key];
        void reconcileScope(scopeType, scopeId);
      }
    }, 2000);
  }
}

export const useNotificationPrefsStore = create<NotificationPrefsState>()(
  (set) => ({
    channelLevels: {},
    dmLevels: {},

    load: async () => {
      const gen = sessionGen;
      const myLoad = ++loadGen;
      const sessionUserId = useAuthStore.getState().user?.id;
      if (!sessionUserId) return;
      const { preferences } = await userAPI.getNotificationPreferences();
      if (
        sessionGen !== gen ||
        myLoad !== loadGen ||
        useAuthStore.getState().user?.id !== sessionUserId
      ) {
        return;
      }
      const channelLevels: Record<string, NotifLevel> = {};
      const dmLevels: Record<string, NotifLevel> = {};
      const preserveChannel = new Set<string>();
      const preserveDm = new Set<string>();
      const seenKeys = new Set<string>();
      for (const p of preferences) {
        if (!isLevel(p.level)) continue;
        if (p.scopeType !== "channel" && p.scopeType !== "dm") continue;
        const key = `${p.scopeType}:${p.scopeId}`;
        seenKeys.add(key);
        if (isStaleVersion(key, p.version)) {
          if (p.scopeType === "channel") preserveChannel.add(p.scopeId);
          else preserveDm.add(p.scopeId);
          continue;
        }
        advanceRemoteVersion(key, p.version);
        if (p.scopeType === "channel") {
          channelLevels[p.scopeId] = p.level;
          channelPersisted[p.scopeId] = p.level;
        } else {
          dmLevels[p.scopeId] = p.level;
          dmPersisted[p.scopeId] = p.level;
        }
      }
      for (const [key, count] of Object.entries(activeWrites)) {
        if (count <= 0) continue;
        if (key.startsWith("channel:")) {
          preserveChannel.add(key.slice("channel:".length));
        } else if (key.startsWith("dm:")) {
          preserveDm.add(key.slice("dm:".length));
        }
      }
      for (const key of Object.keys(remoteVersions)) {
        if (seenKeys.has(key)) continue;
        if (key.startsWith("channel:")) {
          preserveChannel.add(key.slice("channel:".length));
        } else if (key.startsWith("dm:")) {
          preserveDm.add(key.slice("dm:".length));
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
      prefsLoaded = true;
    },

    setChannelLevel: async (channelId, level) => {
      const gen = sessionGen;
      const sessionUserId = useAuthStore.getState().user?.id;
      const seq = (channelSeqs[channelId] = (channelSeqs[channelId] ?? 0) + 1);
      const key = `channel:${channelId}`;
      activeWrites[key] = (activeWrites[key] ?? 0) + 1;
      set((s) => ({
        channelLevels: { ...s.channelLevels, [channelId]: level },
      }));
      const current = () =>
        sessionGen === gen &&
        channelSeqs[channelId] === seq &&
        useAuthStore.getState().user?.id === sessionUserId;
      try {
        await enqueueWrite(key, async () => {
          if (!current()) return;
          try {
            const version = await userAPI.setNotificationPreference(
              "channel",
              channelId,
              level,
            );
            if (sessionGen === gen) {
              channelPersisted[channelId] = level;
              advanceRemoteVersion(key, version);
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
          }
        });
      } finally {
        releaseWrite(key, "channel", channelId);
      }
    },

    setDmLevel: async (friendId, level) => {
      const gen = sessionGen;
      const sessionUserId = useAuthStore.getState().user?.id;
      const seq = (dmSeqs[friendId] = (dmSeqs[friendId] ?? 0) + 1);
      const key = `dm:${friendId}`;
      activeWrites[key] = (activeWrites[key] ?? 0) + 1;
      set((s) => ({ dmLevels: { ...s.dmLevels, [friendId]: level } }));
      const current = () =>
        sessionGen === gen &&
        dmSeqs[friendId] === seq &&
        useAuthStore.getState().user?.id === sessionUserId;
      try {
        await enqueueWrite(key, async () => {
          if (!current()) return;
          try {
            const version = await userAPI.setNotificationPreference(
              "dm",
              friendId,
              level,
            );
            if (sessionGen === gen) {
              dmPersisted[friendId] = level;
              advanceRemoteVersion(key, version);
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
          }
        });
      } finally {
        releaseWrite(key, "dm", friendId);
      }
    },

    applyRemote: (scopeType, scopeId, level, version) => {
      if (!isLevel(level)) return;
      if (scopeType !== "channel" && scopeType !== "dm") return;
      const key = `${scopeType}:${scopeId}`;
      if (
        typeof version === "number" &&
        remoteVersions[key] !== undefined &&
        version <= remoteVersions[key]
      ) {
        return;
      }
      if ((activeWrites[key] ?? 0) > 0) {
        deferredRemote[key] = true;
        return;
      }
      advanceRemoteVersion(key, version);
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
      prefsLoaded = false;
      for (const k of Object.keys(channelSeqs)) delete channelSeqs[k];
      for (const k of Object.keys(dmSeqs)) delete dmSeqs[k];
      for (const k of Object.keys(channelPersisted)) delete channelPersisted[k];
      for (const k of Object.keys(dmPersisted)) delete dmPersisted[k];
      for (const k of Object.keys(writeQueues)) delete writeQueues[k];
      for (const k of Object.keys(activeWrites)) delete activeWrites[k];
      for (const k of Object.keys(deferredRemote)) delete deferredRemote[k];
      for (const k of Object.keys(remoteVersions)) delete remoteVersions[k];
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
