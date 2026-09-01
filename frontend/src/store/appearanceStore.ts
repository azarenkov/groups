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
let saveChain: Promise<unknown> = Promise.resolve();

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

let saveAttempts = 0;

const flushSave = () => {
  const patch = pendingPatch;
  pendingPatch = {};
  const body = toServerPatch(patch);
  if (Object.keys(body).length === 0) return;
  saveChain = saveChain
    .then(() => import("../api/user"))
    .then(({ userAPI }) => userAPI.updateAppearance(body))
    .then(() => {
      saveAttempts = 0;
    })
    .catch((err) => {
      console.warn("[appearance] save failed:", err);
      pendingPatch = { ...patch, ...pendingPatch };
      saveAttempts += 1;
      if (saveAttempts <= 5 && saveTimer === null) {
        saveTimer = window.setTimeout(() => {
          saveTimer = null;
          flushSave();
        }, 5000);
      }
    });
};

const scheduleSave = (patch: Partial<AppearanceSettings>) => {
  pendingPatch = { ...pendingPatch, ...patch };
  if (saveTimer !== null) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    saveTimer = null;
    flushSave();
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
        set(patch);
        const { mode, accent, chatFontSize, reduceMotion } = get();
        applyAppearance({ mode, accent, chatFontSize, reduceMotion });
        scheduleSave(patch);
      },

      syncFromServer: async (userId) => {
        const { userAPI } = await import("../api/user");
        try {
          const saved = await userAPI.getAppearance();
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
              : {
                  mode: "system",
                  accent: DEFAULT_ACCENT,
                  chatFontSize: 14,
                  reduceMotion: false,
                };
            set({ ...local, ownerId: userId });
            applyAppearance(local);
            if (sameOwner) scheduleSave(local);
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
