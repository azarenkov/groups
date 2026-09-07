import { create } from "zustand";
import { persist } from "zustand/middleware";

import { ApiError } from "../utils/apiError";
import { useAuthStore } from "./authStore";

export type MentionLevel = "all" | "mentions" | "none";

export interface NotificationSyncedSettings {
  desktopEnabled: boolean;
  soundsEnabled: boolean;
  mentions: MentionLevel;
  quietHoursEnabled: boolean;
  quietStart: string;
  quietEnd: string;
  callsEnabled: boolean;
}

interface NotificationSettingsState extends NotificationSyncedSettings {
  ownerId: string | null;
  syncedOwner: string | null;
  update: (patch: Partial<NotificationSyncedSettings>) => void;
  syncFromServer: (userId: string) => Promise<void>;
}

const SETTING_KEYS = [
  "desktopEnabled",
  "soundsEnabled",
  "mentions",
  "quietHoursEnabled",
  "quietStart",
  "quietEnd",
  "callsEnabled",
] as const;

const DEFAULT_SETTINGS: NotificationSyncedSettings = {
  desktopEnabled: true,
  soundsEnabled: true,
  mentions: "all",
  quietHoursEnabled: false,
  quietStart: "22:00",
  quietEnd: "08:00",
  callsEnabled: true,
};

const isMentionLevel = (v: unknown): v is MentionLevel =>
  v === "all" || v === "mentions" || v === "none";

const isTime = (v: unknown): v is string =>
  typeof v === "string" && /^([01]\d|2[0-3]):[0-5]\d$/.test(v);

let saveTimer: number | null = null;
let pendingPatch: Partial<NotificationSyncedSettings> = {};
let pendingOwner: string | null = null;

const PENDING_KEY = "rift-notification-settings-pending";

type PendingMap = Record<string, Partial<NotificationSyncedSettings>>;

const readPendingMap = (): PendingMap => {
  try {
    const raw = window.localStorage.getItem(PENDING_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return parsed as PendingMap;
  } catch {
    return {};
  }
};

const writePendingMap = (map: PendingMap) => {
  try {
    if (Object.keys(map).length > 0) {
      window.localStorage.setItem(PENDING_KEY, JSON.stringify(map));
    } else {
      window.localStorage.removeItem(PENDING_KEY);
    }
  } catch {}
};

const stashPatchFor = (
  owner: string,
  patch: Partial<NotificationSyncedSettings>,
) => {
  if (Object.keys(patch).length === 0) return;
  const map = readPendingMap();
  map[owner] = { ...patch, ...(map[owner] ?? {}) };
  writePendingMap(map);
};

const ackPatchFor = (
  owner: string,
  patch: Partial<NotificationSyncedSettings>,
) => {
  const map = readPendingMap();
  const entry = map[owner];
  if (!entry) return;
  const remaining: Partial<NotificationSyncedSettings> = { ...entry };
  for (const key of SETTING_KEYS) {
    if (patch[key] !== undefined && remaining[key] === patch[key]) {
      delete remaining[key];
    }
  }
  if (Object.keys(remaining).length > 0) {
    map[owner] = remaining;
  } else {
    delete map[owner];
  }
  writePendingMap(map);
};

const persistPending = () => {
  if (pendingOwner === null) return;
  try {
    const map = readPendingMap();
    const combined =
      inFlightOwner === pendingOwner
        ? { ...inFlightPatch, ...pendingPatch }
        : pendingPatch;
    if (Object.keys(combined).length > 0) {
      map[pendingOwner] = combined;
    } else {
      delete map[pendingOwner];
    }
    if (Object.keys(map).length > 0) {
      window.localStorage.setItem(PENDING_KEY, JSON.stringify(map));
    } else {
      window.localStorage.removeItem(PENDING_KEY);
    }
  } catch {}
};

const restorePendingFor = (owner: string) => {
  pendingOwner = owner;
  const stored = readPendingMap()[owner];
  pendingPatch = stored && typeof stored === "object" ? { ...stored } : {};
};
let inFlight = false;
let inFlightPatch: Partial<NotificationSyncedSettings> = {};
let inFlightOwner: string | null = null;
let saveAttempts = 0;
let savePaused = false;
let syncRetryTimer: number | null = null;
let syncRetryAttempts = 0;
let syncRetryInFlight = false;
let editSeq = 0;
let syncSeq = 0;
const editStamp: Record<(typeof SETTING_KEYS)[number], number> = {
  desktopEnabled: 0,
  soundsEnabled: 0,
  mentions: 0,
  quietHoursEnabled: 0,
  quietStart: 0,
  quietEnd: 0,
  callsEnabled: 0,
};

const cancelPendingSaves = () => {
  if (saveTimer !== null) {
    window.clearTimeout(saveTimer);
    saveTimer = null;
  }
  pendingPatch = {};
  saveAttempts = 0;
};

const flushSave = async (): Promise<void> => {
  if (inFlight) return;
  const patch = pendingPatch;
  const owner = pendingOwner;
  if (Object.keys(patch).length === 0) return;
  inFlight = true;
  inFlightPatch = patch;
  inFlightOwner = owner;
  pendingPatch = {};
  persistPending();
  try {
    const { useAuthStore } = await import("./authStore");
    const currentId = useAuthStore.getState().user?.id ?? null;
    if (owner !== null && currentId !== owner) {
      if (pendingOwner === owner) {
        pendingPatch = { ...patch, ...pendingPatch };
        persistPending();
        savePaused = true;
      } else {
        stashPatchFor(owner, patch);
      }
      return;
    }
    const { userAPI } = await import("../api/user");
    await userAPI.updateNotificationSettings(patch);
    if (owner !== null) ackPatchFor(owner, patch);
    saveAttempts = 0;
  } catch (err) {
    console.warn("[notification-settings] save failed:", err);
    if (owner === pendingOwner) {
      pendingPatch = { ...patch, ...pendingPatch };
      persistPending();
      saveAttempts += 1;
      if (err instanceof ApiError && err.status === 401) {
        try {
          const { authAPI } = await import("../api/auth");
          await authAPI.handleRefreshFlow();
        } catch {}
      }
      if (saveTimer === null) {
        const delay = Math.min(60_000, 5_000 * 2 ** Math.min(saveAttempts - 1, 4));
        saveTimer = window.setTimeout(() => {
          saveTimer = null;
          void flushSave();
        }, delay);
      }
    } else if (owner !== null) {
      stashPatchFor(owner, patch);
    }
  } finally {
    inFlight = false;
    inFlightPatch = {};
    inFlightOwner = null;
    persistPending();
    if (
      !savePaused &&
      saveAttempts === 0 &&
      saveTimer === null &&
      Object.keys(pendingPatch).length > 0
    ) {
      void flushSave();
    }
  }
};

const scheduleSave = (
  patch: Partial<NotificationSyncedSettings>,
  owner: string | null,
) => {
  savePaused = false;
  if (owner !== pendingOwner) {
    cancelPendingSaves();
    if (owner !== null) {
      restorePendingFor(owner);
    } else {
      pendingOwner = null;
    }
  }
  pendingPatch = { ...pendingPatch, ...patch };
  persistPending();
  saveAttempts = 0;
  if (saveTimer !== null) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    saveTimer = null;
    void flushSave();
  }, 500);
};

export const useNotificationSettingsStore = create<NotificationSettingsState>()(
  persist(
    (set, get) => ({
      ...DEFAULT_SETTINGS,
      ownerId: null,
      syncedOwner: null,

      update: (patch) => {
        editSeq += 1;
        for (const key of SETTING_KEYS) {
          if (patch[key] !== undefined) editStamp[key] = editSeq;
        }
        const prevMentions = get().mentions;
        set(patch);
        const owner = useAuthStore.getState().user?.id ?? get().ownerId;
        scheduleSave(patch, owner);
        if (patch.mentions !== undefined && patch.mentions !== prevMentions) {
          void import("./unreadStore").then((m) => m.refetchUnread().catch(() => {}));
        }
      },

      syncFromServer: async (userId) => {
        if (syncRetryTimer !== null) {
          window.clearTimeout(syncRetryTimer);
          syncRetryTimer = null;
        }
        if (!syncRetryInFlight) syncRetryAttempts = 0;
        syncRetryInFlight = false;
        const { userAPI } = await import("../api/user");
        const prevOwner = get().ownerId;
        const switched = prevOwner !== null && prevOwner !== userId;
        if (prevOwner !== userId) {
          cancelPendingSaves();
          restorePendingFor(userId);
          if (switched) {
            set({ ...DEFAULT_SETTINGS, ...pendingPatch, ownerId: userId, syncedOwner: null });
          } else {
            set({ ...pendingPatch, ownerId: userId, syncedOwner: null });
          }
        } else if (pendingOwner === null) {
          restorePendingFor(userId);
          if (Object.keys(pendingPatch).length > 0) set({ ...pendingPatch });
        }
        const startSeq = editSeq;
        const mySync = ++syncSeq;
        const preservedKeys = new Set<(typeof SETTING_KEYS)[number]>();
        for (const key of SETTING_KEYS) {
          if (
            (pendingPatch[key] !== undefined && pendingOwner === userId) ||
            (inFlightPatch[key] !== undefined && inFlightOwner === userId)
          ) {
            preservedKeys.add(key);
          }
        }
        const preserveRecentEdits = (base: NotificationSyncedSettings) => {
          const merged = { ...base };
          const current = get();
          for (const key of SETTING_KEYS) {
            if (editStamp[key] > startSeq || preservedKeys.has(key)) {
              Object.assign(merged, { [key]: current[key] });
            }
          }
          return merged;
        };
        try {
          const saved = await userAPI.getNotificationSettings();
          const { useAuthStore } = await import("./authStore");
          const authedId = useAuthStore.getState().user?.id ?? null;
          if (syncSeq !== mySync || authedId !== userId) return;
          if (!saved) {
            const local = preserveRecentEdits(
              switched
                ? { ...DEFAULT_SETTINGS }
                : {
                    desktopEnabled: get().desktopEnabled,
                    soundsEnabled: get().soundsEnabled,
                    mentions: get().mentions,
                    quietHoursEnabled: get().quietHoursEnabled,
                    quietStart: get().quietStart,
                    quietEnd: get().quietEnd,
                    callsEnabled: get().callsEnabled,
                  },
            );
            set({ ...local, ownerId: userId, syncedOwner: userId });
            if (!switched) scheduleSave(local, userId);
            else if (Object.keys(pendingPatch).length > 0) scheduleSave({}, userId);
            syncRetryAttempts = 0;
            return;
          }
          const next = preserveRecentEdits({
            desktopEnabled: saved.desktopEnabled,
            soundsEnabled: saved.soundsEnabled,
            mentions: isMentionLevel(saved.mentions) ? saved.mentions : "all",
            quietHoursEnabled: saved.quietHoursEnabled,
            quietStart: isTime(saved.quietStart) ? saved.quietStart : "22:00",
            quietEnd: isTime(saved.quietEnd) ? saved.quietEnd : "08:00",
            callsEnabled: saved.callsEnabled,
          });
          const prevMentions = get().mentions;
          set({ ...next, ownerId: userId, syncedOwner: userId });
          if (next.mentions !== prevMentions) {
            void import("./unreadStore").then((m) => m.refetchUnread().catch(() => {}));
          }
          if (pendingOwner === userId && Object.keys(pendingPatch).length > 0) {
            scheduleSave({}, userId);
          }
          syncRetryAttempts = 0;
        } catch (err) {
          console.warn("[notification-settings] sync failed:", err);
          const { useAuthStore } = await import("./authStore");
          const authedId = useAuthStore.getState().user?.id ?? null;
          if (syncSeq !== mySync || authedId !== userId) return;
          if (pendingOwner === userId && Object.keys(pendingPatch).length > 0) {
            scheduleSave({}, userId);
          }
          if (syncRetryTimer === null) {
            const delay = Math.min(60_000, 5_000 * 2 ** Math.min(syncRetryAttempts, 4));
            syncRetryAttempts += 1;
            syncRetryTimer = window.setTimeout(() => {
              syncRetryTimer = null;
              syncRetryInFlight = true;
              void get().syncFromServer(userId);
            }, delay);
          }
        }
      },
    }),
    {
      name: "rift-notification-settings",
      partialize: ({ syncedOwner: _synced, ...rest }) => rest,
    },
  ),
);

const parseMinutes = (value: string): number => {
  const [h, m] = value.split(":").map(Number);
  return (Number.isFinite(h) ? h : 0) * 60 + (Number.isFinite(m) ? m : 0);
};

export const isQuietNow = (): boolean => {
  const s = useNotificationSettingsStore.getState();
  if (!s.quietHoursEnabled) return false;
  const start = parseMinutes(s.quietStart);
  const end = parseMinutes(s.quietEnd);
  const now = new Date();
  const cur = now.getHours() * 60 + now.getMinutes();
  if (start === end) return true;
  return start < end ? cur >= start && cur < end : cur >= start || cur < end;
};

export const defaultMentionLevel = (): MentionLevel =>
  useNotificationSettingsStore.getState().mentions;

export const invalidateNotificationSettingsSync = (): void => {
  useNotificationSettingsStore.setState({ syncedOwner: null });
};

export const settingsSyncedForCurrentUser = (): boolean => {
  const userId = useAuthStore.getState().user?.id ?? null;
  return userId !== null && useNotificationSettingsStore.getState().syncedOwner === userId;
};

export const messageSoundAllowed = (): boolean =>
  settingsSyncedForCurrentUser() &&
  useNotificationSettingsStore.getState().soundsEnabled &&
  !isQuietNow();

export const desktopNotifAllowed = (): boolean =>
  settingsSyncedForCurrentUser() &&
  useNotificationSettingsStore.getState().desktopEnabled &&
  !isQuietNow();

export const callAlertsAllowed = (): boolean =>
  settingsSyncedForCurrentUser() &&
  useNotificationSettingsStore.getState().callsEnabled;

if (typeof window !== "undefined") {
  window.addEventListener("online", () => {
    const userId = useAuthStore.getState().user?.id ?? null;
    if (!userId) return;
    const state = useNotificationSettingsStore.getState();
    if (pendingOwner === userId && Object.keys(pendingPatch).length > 0) {
      if (saveTimer !== null) {
        window.clearTimeout(saveTimer);
        saveTimer = null;
      }
      savePaused = false;
      saveAttempts = 0;
      void flushSave();
    }
    if (state.syncedOwner === userId) return;
    if (syncRetryTimer !== null) {
      window.clearTimeout(syncRetryTimer);
      syncRetryTimer = null;
    }
    syncRetryInFlight = true;
    void state.syncFromServer(userId);
  });
}
