import { create } from "zustand";
import { persist } from "zustand/middleware";

import { getLiveKitService } from "../lib/livekit";
import { ApiError } from "../../../shared/lib/apiError";

export type NoiseMode = "off" | "standard" | "high";
export type StreamQualityId = "720p60" | "1080p60" | "1440p60" | "2160p60";

const parseStreamQuality = (value: unknown): StreamQualityId =>
  value === "1080p60" || value === "1440p60" || value === "2160p60" ? value : "720p60";

export interface VoiceSyncedSettings {
  micVolume: number;
  outputVolume: number;
  noiseMode: NoiseMode;
  autoSensitivity: boolean;
  sensitivity: number;
  pushToTalk: boolean;
  pttKey: string;
  streamQuality: StreamQualityId;
}

interface VoiceSettingsState extends VoiceSyncedSettings {
  micDeviceId: string;
  speakerDeviceId: string;
  pttGlobal: boolean;
  ownerId: string | null;
  update: (patch: Partial<VoiceSyncedSettings>) => void;
  setPttGlobal: (value: boolean) => void;
  updateDevices: (patch: {
    micDeviceId?: string;
    speakerDeviceId?: string;
  }) => void;
  syncFromServer: (userId: string) => Promise<void>;
}

const SETTING_KEYS = [
  "micVolume",
  "outputVolume",
  "noiseMode",
  "autoSensitivity",
  "sensitivity",
  "pushToTalk",
  "pttKey",
  "streamQuality",
] as const;

const DEFAULT_SETTINGS: VoiceSyncedSettings = {
  micVolume: 100,
  outputVolume: 100,
  noiseMode: "standard",
  autoSensitivity: true,
  sensitivity: 50,
  pushToTalk: false,
  pttKey: "KeyM",
  streamQuality: "720p60",
};

const applyToEngine = (s: {
  micDeviceId: string;
  speakerDeviceId: string;
  outputVolume: number;
  micVolume: number;
  noiseMode: NoiseMode;
}) => {
  getLiveKitService().setAudioPreferences({
    micDeviceId: s.micDeviceId || null,
    speakerDeviceId: s.speakerDeviceId || null,
    outputVolume: s.outputVolume / 100,
    micVolume: s.micVolume / 100,
    noiseMode: s.noiseMode,
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
  if (syncRetryTimer !== null) {
    window.clearTimeout(syncRetryTimer);
    syncRetryTimer = null;
  }
  syncRetryAttempts = 0;
};

export const awaitVoiceSettingsSyncedFor = async (
  userId: string | null,
  timeoutMs = 4000,
): Promise<boolean> => {
  if (!userId) return true;
  return await Promise.race([
    voiceSettingsReadyFor(userId).then(() => true),
    new Promise<boolean>((resolve) =>
      setTimeout(() => resolve(false), timeoutMs),
    ),
  ]);
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
let syncRetryTimer: number | null = null;
let syncRetryAttempts = 0;
let syncRetryInFlight = false;
let editSeq = 0;
let syncSeq = 0;
const editStamp: Record<(typeof SETTING_KEYS)[number], number> = {
  micVolume: 0,
  outputVolume: 0,
  noiseMode: 0,
  autoSensitivity: 0,
  sensitivity: 0,
  pushToTalk: 0,
  pttKey: 0,
  streamQuality: 0,
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
    const [{ useAuthStore }, { userAPI }] = await Promise.all([
      import("../../auth/store/authStore"),
      import("../../social/api/user"),
    ]);
    const auth = useAuthStore.getState();
    const currentId = auth.user?.id ?? null;
    const accessToken = auth.tokens?.accessToken ?? null;
    if (owner !== null && currentId !== owner) {
      if (pendingOwner === owner) {
        pendingPatch = { ...patch, ...pendingPatch };
        savePaused = true;
      }
      return;
    }
    await userAPI.updateVoiceSettings(patch, accessToken ?? undefined);
    saveAttempts = 0;
  } catch (err) {
    console.warn("[voice-settings] save failed:", err);
    if (owner === pendingOwner) {
      pendingPatch = { ...patch, ...pendingPatch };
      saveAttempts += 1;
      if (err instanceof ApiError && err.status === 401) {
        try {
          const { authAPI } = await import("../../auth/api/auth");
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
      pttGlobal: true,
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

      setPttGlobal: (value) => {
        set({ pttGlobal: value });
      },

      updateDevices: (patch) => {
        set(patch);
        applyToEngine(get());
      },

      syncFromServer: async (userId) => {
        syncedUsers.delete(userId);
        if (syncRetryTimer !== null) {
          window.clearTimeout(syncRetryTimer);
          syncRetryTimer = null;
        }
        if (!syncRetryInFlight) syncRetryAttempts = 0;
        syncRetryInFlight = false;
        const { userAPI } = await import("../../social/api/user");
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
          const { useAuthStore } = await import("../../auth/store/authStore");
          const authedId = useAuthStore.getState().user?.id ?? null;
          if (syncSeq !== mySync || authedId !== userId) return;
          if (!saved) {
            const local = preserveRecentEdits(
              switched
                ? { ...DEFAULT_SETTINGS }
                : {
                    micVolume: get().micVolume,
                    outputVolume: get().outputVolume,
                    noiseMode: get().noiseMode,
                    autoSensitivity: get().autoSensitivity,
                    sensitivity: get().sensitivity,
                    pushToTalk: get().pushToTalk,
                    pttKey: get().pttKey,
                    streamQuality: get().streamQuality,
                  },
            );
            set({ ...local, ownerId: userId });
            applyToEngine(get());
            if (!switched) scheduleSave(local, userId);
            syncRetryAttempts = 0;
            markUserSynced(userId);
            return;
          }
          const next = preserveRecentEdits({
            micVolume: saved.micVolume,
            outputVolume: saved.outputVolume,
            noiseMode:
              saved.noiseMode === "off" || saved.noiseMode === "high"
                ? saved.noiseMode
                : "standard",
            autoSensitivity: saved.autoSensitivity,
            sensitivity: saved.sensitivity,
            pushToTalk: saved.pushToTalk,
            pttKey: saved.pttKey || "KeyM",
            streamQuality: parseStreamQuality(saved.streamQuality),
          });
          set({ ...next, ownerId: userId });
          applyToEngine(get());
          if (pendingOwner === userId && Object.keys(pendingPatch).length > 0) {
            scheduleSave({}, userId);
          }
          syncRetryAttempts = 0;
          markUserSynced(userId);
        } catch (err) {
          console.warn("[voice-settings] sync failed:", err);
          const { useAuthStore } = await import("../../auth/store/authStore");
          const authedId = useAuthStore.getState().user?.id ?? null;
          if (syncSeq !== mySync || authedId !== userId) return;
          applyToEngine(get());
          if (pendingOwner === userId && Object.keys(pendingPatch).length > 0) {
            scheduleSave({}, userId);
          }
          if (syncRetryAttempts < 5 && syncRetryTimer === null) {
            syncRetryAttempts += 1;
            syncRetryTimer = window.setTimeout(() => {
              syncRetryTimer = null;
              syncRetryInFlight = true;
              void get().syncFromServer(userId);
            }, 5000);
          }
        }
      },
    }),
    { name: "rift-voice-settings" },
  ),
);

applyToEngine(useVoiceSettingsStore.getState());

if (typeof navigator !== "undefined" && navigator.mediaDevices?.addEventListener) {
  navigator.mediaDevices.addEventListener("devicechange", () => {
    navigator.mediaDevices
      .enumerateDevices()
      .then((list) => {
        if (!list.some((d) => d.label)) return;
        const state = useVoiceSettingsStore.getState();
        const patch: { micDeviceId?: string; speakerDeviceId?: string } = {};
        if (
          state.micDeviceId &&
          !list.some((d) => d.kind === "audioinput" && d.deviceId === state.micDeviceId)
        ) {
          patch.micDeviceId = "";
        }
        if (
          state.speakerDeviceId &&
          !list.some((d) => d.kind === "audiooutput" && d.deviceId === state.speakerDeviceId)
        ) {
          patch.speakerDeviceId = "";
        }
        if (Object.keys(patch).length > 0) state.updateDevices(patch);
      })
      .catch(() => {});
  });
}
