import { create } from "zustand";
import { persist } from "zustand/middleware";

import { getLiveKitService } from "../services/livekit";
import { ApiError } from "../utils/apiError";

export interface VoiceSyncedSettings {
  micVolume: number;
  outputVolume: number;
  noiseSuppression: boolean;
  autoSensitivity: boolean;
  sensitivity: number;
  pushToTalk: boolean;
}

interface VoiceSettingsState extends VoiceSyncedSettings {
  micDeviceId: string;
  speakerDeviceId: string;
  ownerId: string | null;
  update: (patch: Partial<VoiceSyncedSettings>) => void;
  updateDevices: (patch: {
    micDeviceId?: string;
    speakerDeviceId?: string;
  }) => void;
  syncFromServer: (userId: string) => Promise<void>;
}

const SETTING_KEYS = [
  "micVolume",
  "outputVolume",
  "noiseSuppression",
  "autoSensitivity",
  "sensitivity",
  "pushToTalk",
] as const;

const DEFAULT_SETTINGS: VoiceSyncedSettings = {
  micVolume: 100,
  outputVolume: 100,
  noiseSuppression: true,
  autoSensitivity: true,
  sensitivity: 50,
  pushToTalk: false,
};

const applyToEngine = (s: {
  micDeviceId: string;
  speakerDeviceId: string;
  outputVolume: number;
  micVolume: number;
  noiseSuppression: boolean;
}) => {
  getLiveKitService().setAudioPreferences({
    micDeviceId: s.micDeviceId || null,
    speakerDeviceId: s.speakerDeviceId || null,
    outputVolume: s.outputVolume / 100,
    micVolume: s.micVolume / 100,
    noiseSuppression: s.noiseSuppression,
  });
};

const syncedUsers = new Set<string>();
let syncWaiters: { userId: string; resolve: () => void }[] = [];

export const voiceSettingsReadyFor = (userId: string): Promise<void> => {
  if (syncedUsers.has(userId)) return Promise.resolve();
  return new Promise((resolve) => {
    syncWaiters.push({ userId, resolve });
  });
};

export const invalidateVoiceSettingsSync = (): void => {
  syncedUsers.clear();
};

const markUserSynced = (userId: string) => {
  syncedUsers.add(userId);
  syncWaiters = syncWaiters.filter((waiter) => {
    if (waiter.userId !== userId) return true;
    waiter.resolve();
    return false;
  });
};

let saveTimer: number | null = null;
let pendingPatch: Partial<VoiceSyncedSettings> = {};
let pendingOwner: string | null = null;
let inFlight = false;
let inFlightPatch: Partial<VoiceSyncedSettings> = {};
let inFlightOwner: string | null = null;
let saveAttempts = 0;
let savePaused = false;
let editSeq = 0;
let syncSeq = 0;
const editStamp: Record<(typeof SETTING_KEYS)[number], number> = {
  micVolume: 0,
  outputVolume: 0,
  noiseSuppression: 0,
  autoSensitivity: 0,
  sensitivity: 0,
  pushToTalk: 0,
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
  pendingPatch = {};
  if (Object.keys(patch).length === 0) return;
  inFlight = true;
  inFlightPatch = patch;
  inFlightOwner = owner;
  try {
    const { useAuthStore } = await import("./authStore");
    const currentId = useAuthStore.getState().user?.id ?? null;
    if (owner !== null && currentId !== owner) {
      pendingPatch = { ...patch, ...pendingPatch };
      savePaused = true;
      return;
    }
    const { userAPI } = await import("../api/user");
    await userAPI.updateVoiceSettings(patch);
    saveAttempts = 0;
  } catch (err) {
    console.warn("[voice-settings] save failed:", err);
    if (owner === pendingOwner) {
      pendingPatch = { ...patch, ...pendingPatch };
      saveAttempts += 1;
      if (err instanceof ApiError && err.status === 401) {
        try {
          const { authAPI } = await import("../api/auth");
          await authAPI.handleRefreshFlow();
        } catch {}
      }
      if (saveAttempts <= 5 && saveTimer === null) {
        saveTimer = window.setTimeout(() => {
          saveTimer = null;
          void flushSave();
        }, 5000);
      }
    }
  } finally {
    inFlight = false;
    inFlightPatch = {};
    inFlightOwner = null;
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
  patch: Partial<VoiceSyncedSettings>,
  owner: string | null,
) => {
  savePaused = false;
  if (owner !== pendingOwner) {
    cancelPendingSaves();
    pendingOwner = owner;
  }
  pendingPatch = { ...pendingPatch, ...patch };
  saveAttempts = 0;
  if (saveTimer !== null) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    saveTimer = null;
    void flushSave();
  }, 500);
};

export const useVoiceSettingsStore = create<VoiceSettingsState>()(
  persist(
    (set, get) => ({
      ...DEFAULT_SETTINGS,
      micDeviceId: "",
      speakerDeviceId: "",
      ownerId: null,

      update: (patch) => {
        editSeq += 1;
        for (const key of SETTING_KEYS) {
          if (patch[key] !== undefined) editStamp[key] = editSeq;
        }
        set(patch);
        const state = get();
        applyToEngine(state);
        scheduleSave(patch, state.ownerId);
      },

      updateDevices: (patch) => {
        set(patch);
        applyToEngine(get());
      },

      syncFromServer: async (userId) => {
        syncedUsers.delete(userId);
        const { userAPI } = await import("../api/user");
        const prevOwner = get().ownerId;
        const switched = prevOwner !== null && prevOwner !== userId;
        if (prevOwner !== userId) {
          if (switched) {
            cancelPendingSaves();
            set({ ...DEFAULT_SETTINGS, ownerId: userId });
            applyToEngine(get());
          } else {
            set({ ownerId: userId });
          }
          pendingOwner = userId;
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
        const preserveRecentEdits = (base: VoiceSyncedSettings) => {
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
          const saved = await userAPI.getVoiceSettings();
          const { useAuthStore } = await import("./authStore");
          const authedId = useAuthStore.getState().user?.id ?? null;
          if (syncSeq !== mySync || authedId !== userId) return;
          if (!saved) {
            const local = preserveRecentEdits(
              switched
                ? { ...DEFAULT_SETTINGS }
                : {
                    micVolume: get().micVolume,
                    outputVolume: get().outputVolume,
                    noiseSuppression: get().noiseSuppression,
                    autoSensitivity: get().autoSensitivity,
                    sensitivity: get().sensitivity,
                    pushToTalk: get().pushToTalk,
                  },
            );
            set({ ...local, ownerId: userId });
            applyToEngine(get());
            if (!switched) scheduleSave(local, userId);
            return;
          }
          const next = preserveRecentEdits({
            micVolume: saved.micVolume,
            outputVolume: saved.outputVolume,
            noiseSuppression: saved.noiseSuppression,
            autoSensitivity: saved.autoSensitivity,
            sensitivity: saved.sensitivity,
            pushToTalk: saved.pushToTalk,
          });
          set({ ...next, ownerId: userId });
          applyToEngine(get());
          if (pendingOwner === userId && Object.keys(pendingPatch).length > 0) {
            scheduleSave({}, userId);
          }
        } catch (err) {
          console.warn("[voice-settings] sync failed:", err);
          applyToEngine(get());
        } finally {
          markUserSynced(userId);
        }
      },
    }),
    { name: "rift-voice-settings" },
  ),
);

applyToEngine(useVoiceSettingsStore.getState());
