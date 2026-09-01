import { create } from "zustand";
import { persist } from "zustand/middleware";

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
  root.style.setProperty("--chat-font-size", `${s.chatFontSize}px`);
};

let saveTimer: number | null = null;
let pendingPatch: Partial<AppearanceSettings> = {};
let pendingOwner: string | null = null;
let inFlight = false;
let inFlightPatch: Partial<AppearanceSettings> = {};
let saveAttempts = 0;
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
  try {
    const { useAuthStore } = await import("./authStore");
    const currentId = useAuthStore.getState().user?.id ?? null;
    if (owner !== null && currentId !== owner) {
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
    if (
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
  if (owner !== pendingOwner) {
    cancelPendingSaves();
    pendingOwner = owner;
  }
  pendingPatch = { ...pendingPatch, ...patch };
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
        const { userAPI } = await import("../api/user");
        if (get().ownerId !== null && get().ownerId !== userId) {
          cancelPendingSaves();
          pendingOwner = userId;
        }
        const startSeq = editSeq;
        const mySync = ++syncSeq;
        try {
          const saved = await userAPI.getAppearance();
          const { useAuthStore } = await import("./authStore");
          const authedId = useAuthStore.getState().user?.id ?? null;
          if (syncSeq !== mySync || authedId !== userId) return;
          if (!saved) {
            const sameOwner =
              get().ownerId === null || get().ownerId === userId;
            const local: AppearanceSettings = sameOwner
              ? {
                  mode: get().mode,
                  accent: get().accent,
                  chatFontSize: get().chatFontSize,
                  reduceMotion: get().reduceMotion,
                }
              : { ...DEFAULT_SETTINGS };
            set({ ...local, ownerId: userId });
            applyAppearance(local);
            if (sameOwner) scheduleSave(local, userId);
            return;
          }
          const next: AppearanceSettings = {
            mode:
              saved.theme === "light" || saved.theme === "dark"
                ? saved.theme
                : "system",
            accent: saved.accent || DEFAULT_ACCENT,
            chatFontSize: saved.chatFontSize,
            reduceMotion: saved.reduceMotion,
          };
          const current = get();
          for (const key of SETTING_KEYS) {
            if (
              editStamp[key] > startSeq ||
              pendingPatch[key] !== undefined ||
              inFlightPatch[key] !== undefined
            ) {
              Object.assign(next, { [key]: current[key] });
            }
          }
          set({ ...next, ownerId: userId });
          applyAppearance(next);
        } catch (err) {
          console.warn("[appearance] sync failed:", err);
        }
      },
    }),
    { name: "rift-appearance" },
  ),
);
