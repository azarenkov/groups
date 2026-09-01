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
  update: (patch: Partial<AppearanceSettings>) => void;
  syncFromServer: () => Promise<void>;
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

const scheduleSave = (state: AppearanceSettings) => {
  if (saveTimer !== null) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    saveTimer = null;
    void import("../api/user").then(({ userAPI }) =>
      userAPI
        .updateAppearance({
          theme: state.mode,
          accent: state.accent,
          chatFontSize: state.chatFontSize,
          reduceMotion: state.reduceMotion,
        })
        .catch((err) => console.warn("[appearance] save failed:", err)),
    );
  }, 500);
};

export const useAppearanceStore = create<AppearanceState>()(
  persist(
    (set, get) => ({
      mode: legacyMode(),
      accent: DEFAULT_ACCENT,
      chatFontSize: 14,
      reduceMotion: false,

      update: (patch) => {
        set(patch);
        const { mode, accent, chatFontSize, reduceMotion } = get();
        const next = { mode, accent, chatFontSize, reduceMotion };
        applyAppearance(next);
        scheduleSave(next);
      },

      syncFromServer: async () => {
        const { userAPI } = await import("../api/user");
        try {
          const saved = await userAPI.getAppearance();
          const next: AppearanceSettings = {
            mode:
              saved.theme === "light" || saved.theme === "dark"
                ? saved.theme
                : "system",
            accent: saved.accent || DEFAULT_ACCENT,
            chatFontSize: saved.chatFontSize,
            reduceMotion: saved.reduceMotion,
          };
          set(next);
          applyAppearance(next);
        } catch (err) {
          console.warn("[appearance] sync failed:", err);
        }
      },
    }),
    { name: "rift-appearance" },
  ),
);
