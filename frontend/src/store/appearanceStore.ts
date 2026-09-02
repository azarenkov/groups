import { create } from "zustand";
import { persist } from "zustand/middleware";

import { ApiError } from "../utils/apiError";

export type ThemeMode = "light" | "dark" | "system";

export interface AppearanceSettings {
  mode: ThemeMode;
  accent: string;
  chatFontSize: number;
  reduceMotion: boolean;
}

interface AppearanceState extends AppearanceSettings {
  ownerId: string | null;
  update: (patch: Partial<AppearanceSettings>) => void;
  syncFromServer: (userId: string) => Promise<void>;
}

export const DEFAULT_ACCENT = "#0071e3";

const SETTING_KEYS = ["mode", "accent", "chatFontSize", "reduceMotion"] as const;

const DEFAULT_SETTINGS: AppearanceSettings = {
  mode: "system",
  accent: DEFAULT_ACCENT,
  chatFontSize: 14,
  reduceMotion: false,
};

const legacyMode = (): ThemeMode => {
  try {
    const raw = localStorage.getItem("rift-theme");
    if (!raw) return "system";
    const parsed = JSON.parse(raw);
    const t = parsed?.state?.theme;
    return t === "light" || t === "dark" ? t : "system";
  } catch {
    return "system";
  }
};

export const applyAppearance = (s: AppearanceSettings) => {
  const dark =
    s.mode === "dark" ||
    (s.mode === "system" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);
  const root = document.documentElement;
  root.classList.toggle("dark", dark);
  root.classList.toggle("reduce-motion", s.reduceMotion);
  root.style.setProperty("--accent", s.accent);
  root.style.setProperty(
    "--accent-hover",
    `color-mix(in srgb, ${s.accent} 88%, #000)`,
  );
  root.style.setProperty(
    "--accent-tint",
    `color-mix(in srgb, ${s.accent} 14%, transparent)`,
  );
  root.style.setProperty(
    "--accent-tint-2",
    `color-mix(in srgb, ${s.accent} 24%, transparent)`,
  );
  root.style.setProperty(
    "--shadow-focus",
    `0 0 0 4px color-mix(in srgb, ${s.accent} 22%, transparent)`,
  );
  root.style.setProperty("--chat-font-size", `${s.chatFontSize}px`);
};

let saveTimer: number | null = null;
let pendingPatch: Partial<AppearanceSettings> = {};
let pendingOwner: string | null = null;
let inFlight = false;
let inFlightPatch: Partial<AppearanceSettings> = {};
let inFlightOwner: string | null = null;
let saveAttempts = 0;
let savePaused = false;
let syncRetryTimer: number | null = null;
let syncRetryAttempts = 0;
let syncRetryInFlight = false;
let editSeq = 0;
let syncSeq = 0;
const editStamp: Record<(typeof SETTING_KEYS)[number], number> = {
  mode: 0,
  accent: 0,
  chatFontSize: 0,
  reduceMotion: 0,
};

const toServerPatch = (patch: Partial<AppearanceSettings>) => {
  const body: {
    theme?: string;
    accent?: string;
    chatFontSize?: number;
    reduceMotion?: boolean;
  } = {};
  if (patch.mode !== undefined) body.theme = patch.mode;
  if (patch.accent !== undefined) body.accent = patch.accent;
  if (patch.chatFontSize !== undefined) body.chatFontSize = patch.chatFontSize;
  if (patch.reduceMotion !== undefined) body.reduceMotion = patch.reduceMotion;
  return body;
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
  const body = toServerPatch(patch);
  if (Object.keys(body).length === 0) return;
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
    await userAPI.updateAppearance(body);
    saveAttempts = 0;
  } catch (err) {
    console.warn("[appearance] save failed:", err);
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
  patch: Partial<AppearanceSettings>,
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

export const useAppearanceStore = create<AppearanceState>()(
  persist(
    (set, get) => ({
      mode: legacyMode(),
      accent: DEFAULT_ACCENT,
      chatFontSize: 14,
      reduceMotion: false,
      ownerId: null,

      update: (patch) => {
        editSeq += 1;
        for (const key of SETTING_KEYS) {
          if (patch[key] !== undefined) editStamp[key] = editSeq;
        }
        set(patch);
        const { mode, accent, chatFontSize, reduceMotion, ownerId } = get();
        applyAppearance({ mode, accent, chatFontSize, reduceMotion });
        scheduleSave(patch, ownerId);
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
          if (switched) {
            cancelPendingSaves();
            set({ ...DEFAULT_SETTINGS, ownerId: userId });
            applyAppearance(DEFAULT_SETTINGS);
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
        const preserveRecentEdits = (base: AppearanceSettings) => {
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
          const saved = await userAPI.getAppearance();
          const { useAuthStore } = await import("./authStore");
          const authedId = useAuthStore.getState().user?.id ?? null;
          if (syncSeq !== mySync || authedId !== userId) return;
          if (!saved) {
            const local = preserveRecentEdits(
              switched
                ? { ...DEFAULT_SETTINGS }
                : {
                    mode: get().mode,
                    accent: get().accent,
                    chatFontSize: get().chatFontSize,
                    reduceMotion: get().reduceMotion,
                  },
            );
            set({ ...local, ownerId: userId });
            applyAppearance(local);
            if (!switched) scheduleSave(local, userId);
            syncRetryAttempts = 0;
            return;
          }
          const next = preserveRecentEdits({
            mode:
              saved.theme === "light" || saved.theme === "dark"
                ? saved.theme
                : "system",
            accent: saved.accent || DEFAULT_ACCENT,
            chatFontSize: saved.chatFontSize,
            reduceMotion: saved.reduceMotion,
          });
          set({ ...next, ownerId: userId });
          applyAppearance(next);
          if (pendingOwner === userId && Object.keys(pendingPatch).length > 0) {
            scheduleSave({}, userId);
          }
          syncRetryAttempts = 0;
        } catch (err) {
          console.warn("[appearance] sync failed:", err);
          const { useAuthStore } = await import("./authStore");
          const authedId = useAuthStore.getState().user?.id ?? null;
          if (syncSeq !== mySync || authedId !== userId) return;
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
    { name: "rift-appearance" },
  ),
);
