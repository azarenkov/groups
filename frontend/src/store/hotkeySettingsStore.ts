import { create } from "zustand";
import { persist } from "zustand/middleware";

import { ApiError } from "../utils/apiError";
import { HOTKEY_ACTION_IDS, isHotkeyActionId, isValidBinding } from "../utils/hotkeys";
import { useAuthStore } from "./authStore";

export type HotkeyBindings = Record<string, string>;
export type HotkeyPatch = Record<string, string | null>;

interface HotkeySettingsState {
  bindings: HotkeyBindings;
  ownerId: string | null;
  syncedOwner: string | null;
  setBinding: (action: string, binding: string | null) => void;
  resetAll: () => void;
  syncFromServer: (userId: string) => Promise<void>;
}

const PENDING_KEY = "rift-hotkey-settings-pending";

type PendingMap = Record<string, HotkeyPatch>;

const sanitize = (raw: unknown): HotkeyBindings => {
  const out: HotkeyBindings = {};
  if (!raw || typeof raw !== "object") return out;
  for (const [action, binding] of Object.entries(raw as Record<string, unknown>)) {
    if (isHotkeyActionId(action) && isValidBinding(binding)) out[action] = binding;
  }
  return out;
};

const applyPatch = (bindings: HotkeyBindings, patch: HotkeyPatch): HotkeyBindings => {
  const next = { ...bindings };
  for (const [action, binding] of Object.entries(patch)) {
    if (binding === null) delete next[action];
    else next[action] = binding;
  }
  return next;
};

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

const stashPatchFor = (owner: string, patch: HotkeyPatch) => {
  if (Object.keys(patch).length === 0) return;
  const map = readPendingMap();
  map[owner] = { ...patch, ...(map[owner] ?? {}) };
  writePendingMap(map);
};

const ackPatchFor = (owner: string, patch: HotkeyPatch) => {
  const map = readPendingMap();
  const entry = map[owner];
  if (!entry) return;
  const remaining: HotkeyPatch = { ...entry };
  for (const [action, binding] of Object.entries(patch)) {
    if (remaining[action] === binding) delete remaining[action];
  }
  if (Object.keys(remaining).length > 0) map[owner] = remaining;
  else delete map[owner];
  writePendingMap(map);
};

let saveTimer: number | null = null;
let pendingPatch: HotkeyPatch = {};
let pendingOwner: string | null = null;
let inFlight = false;
let inFlightPatch: HotkeyPatch = {};
let inFlightOwner: string | null = null;
let saveAttempts = 0;
let savePaused = false;
let syncRetryTimer: number | null = null;
let syncRetryAttempts = 0;
let syncRetryInFlight = false;
let editSeq = 0;
let syncSeq = 0;
const editStamp: Record<string, number> = {};

const persistPending = () => {
  if (pendingOwner === null) return;
  const map = readPendingMap();
  const combined =
    inFlightOwner === pendingOwner ? { ...inFlightPatch, ...pendingPatch } : pendingPatch;
  if (Object.keys(combined).length > 0) map[pendingOwner] = combined;
  else delete map[pendingOwner];
  writePendingMap(map);
};

const restorePendingFor = (owner: string) => {
  pendingOwner = owner;
  const stored = readPendingMap()[owner];
  pendingPatch = stored && typeof stored === "object" ? { ...stored } : {};
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
    const saved = await userAPI.updateHotkeySettings(patch);
    if (owner !== null) ackPatchFor(owner, patch);
    saveAttempts = 0;
    if (owner !== null && useAuthStore.getState().user?.id === owner) {
      const newer = pendingOwner === owner ? pendingPatch : {};
      useHotkeySettingsStore.setState({
        bindings: applyPatch(sanitize(saved.bindings), newer),
      });
    }
  } catch (err) {
    console.warn("[hotkey-settings] save failed:", err);
    const rejected =
      err instanceof ApiError &&
      (err.code === "HOTKEY_CONFLICT" || err.code === "INVALID_HOTKEY_SETTINGS");
    if (rejected) {
      inFlightPatch = {};
      inFlightOwner = null;
      if (owner !== null) {
        ackPatchFor(owner, patch);
        for (const action of Object.keys(patch)) delete editStamp[action];
        if (useAuthStore.getState().user?.id === owner) {
          void useHotkeySettingsStore.getState().syncFromServer(owner);
        }
      }
      saveAttempts = 0;
    } else if (owner === pendingOwner) {
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

const scheduleSave = (patch: HotkeyPatch, owner: string | null) => {
  savePaused = false;
  if (owner !== pendingOwner) {
    cancelPendingSaves();
    if (owner !== null) restorePendingFor(owner);
    else pendingOwner = null;
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

export const useHotkeySettingsStore = create<HotkeySettingsState>()(
  persist(
    (set, get) => ({
      bindings: {},
      ownerId: null,
      syncedOwner: null,

      setBinding: (action, binding) => {
        if (!isHotkeyActionId(action)) return;
        if (binding !== null && !isValidBinding(binding)) return;
        const userId = useAuthStore.getState().user?.id ?? null;
        if (userId === null) return;
        editSeq += 1;
        editStamp[action] = editSeq;
        const patch: HotkeyPatch = { [action]: binding };
        const owned = get().ownerId === userId ? get().bindings : {};
        set({ bindings: applyPatch(owned, patch), ownerId: userId });
        scheduleSave(patch, userId);
      },

      resetAll: () => {
        const userId = useAuthStore.getState().user?.id ?? null;
        if (userId === null) return;
        editSeq += 1;
        const patch: HotkeyPatch = {};
        for (const action of HOTKEY_ACTION_IDS) {
          editStamp[action] = editSeq;
          patch[action] = null;
        }
        set({ bindings: {}, ownerId: userId });
        scheduleSave(patch, userId);
      },

      syncFromServer: async (userId) => {
        if (get().syncedOwner !== null) set({ syncedOwner: null });
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
          set({
            bindings: applyPatch(switched ? {} : get().bindings, pendingPatch),
            ownerId: userId,
            syncedOwner: null,
          });
        } else if (pendingOwner === null) {
          restorePendingFor(userId);
          if (Object.keys(pendingPatch).length > 0) {
            set({ bindings: applyPatch(get().bindings, pendingPatch) });
          }
        }
        const startSeq = editSeq;
        const mySync = ++syncSeq;
        const preserved = new Set<string>();
        for (const action of Object.keys(pendingPatch)) {
          if (pendingOwner === userId) preserved.add(action);
        }
        for (const action of Object.keys(inFlightPatch)) {
          if (inFlightOwner === userId) preserved.add(action);
        }
        const preserveRecentEdits = (base: HotkeyBindings): HotkeyBindings => {
          const merged = { ...base };
          const current = get().bindings;
          const recent = new Set(preserved);
          for (const [action, stamp] of Object.entries(editStamp)) {
            if (stamp > startSeq) recent.add(action);
          }
          for (const action of recent) {
            if (current[action] !== undefined) merged[action] = current[action];
            else delete merged[action];
          }
          return merged;
        };
        try {
          const saved = await userAPI.getHotkeySettings();
          const authedId = useAuthStore.getState().user?.id ?? null;
          if (syncSeq !== mySync || authedId !== userId) return;
          if (!saved) {
            const local = preserveRecentEdits(switched ? {} : get().bindings);
            set({ bindings: local, ownerId: userId, syncedOwner: userId });
            if (!switched && Object.keys(local).length > 0) {
              scheduleSave({ ...local }, userId);
            } else if (Object.keys(pendingPatch).length > 0) {
              scheduleSave({}, userId);
            }
            syncRetryAttempts = 0;
            return;
          }
          const next = preserveRecentEdits(sanitize(saved.bindings));
          set({ bindings: next, ownerId: userId, syncedOwner: userId });
          if (pendingOwner === userId && Object.keys(pendingPatch).length > 0) {
            scheduleSave({}, userId);
          }
          syncRetryAttempts = 0;
        } catch (err) {
          console.warn("[hotkey-settings] sync failed:", err);
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
      name: "rift-hotkey-settings",
      partialize: ({ syncedOwner: _synced, ...rest }) => rest,
      merge: (persisted, current) => {
        const stored = (persisted ?? {}) as Partial<HotkeySettingsState>;
        return {
          ...current,
          ...stored,
          bindings: sanitize(stored.bindings),
          syncedOwner: null,
        };
      },
    },
  ),
);

export const invalidateHotkeySettingsSync = (): void => {
  syncSeq += 1;
  if (syncRetryTimer !== null) {
    window.clearTimeout(syncRetryTimer);
    syncRetryTimer = null;
  }
  syncRetryAttempts = 0;
  syncRetryInFlight = false;
  useHotkeySettingsStore.setState({ syncedOwner: null });
};

if (typeof window !== "undefined") {
  window.addEventListener("online", () => {
    const userId = useAuthStore.getState().user?.id ?? null;
    if (!userId) return;
    if (pendingOwner === userId && Object.keys(pendingPatch).length > 0) {
      if (saveTimer !== null) {
        window.clearTimeout(saveTimer);
        saveTimer = null;
      }
      savePaused = false;
      saveAttempts = 0;
      void flushSave();
    }
    if (syncRetryTimer !== null) {
      window.clearTimeout(syncRetryTimer);
      syncRetryTimer = null;
    }
    syncRetryInFlight = true;
    void useHotkeySettingsStore.getState().syncFromServer(userId);
  });
}
