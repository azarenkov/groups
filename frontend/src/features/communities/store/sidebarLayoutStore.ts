import { create } from "zustand";

export const SIDEBAR_COLLAPSED_WIDTH = 72;
export const SIDEBAR_MIN_WIDTH = 240;
export const SIDEBAR_MAX_WIDTH = 420;
export const SIDEBAR_DEFAULT_WIDTH = 320;
export const SIDEBAR_SNAP_WIDTH = 160;
const STORAGE_KEY = "rift.sidebarLayout";

interface SidebarLayoutState {
  width: number;
  collapsed: boolean;
  setCollapsed: (collapsed: boolean) => void;
  toggle: () => void;
  dragTo: (width: number) => void;
  nudge: (delta: number) => void;
  persist: () => void;
}

const clampWidth = (width: number) => Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, Math.round(width)));

const readStored = (): { width: number; collapsed: boolean } => {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as { width?: unknown; collapsed?: unknown };
      return {
        width: typeof parsed.width === "number" ? clampWidth(parsed.width) : SIDEBAR_DEFAULT_WIDTH,
        collapsed: parsed.collapsed === true,
      };
    }
  } catch {
    return { width: SIDEBAR_DEFAULT_WIDTH, collapsed: false };
  }
  return { width: SIDEBAR_DEFAULT_WIDTH, collapsed: false };
};

const writeStored = (width: number, collapsed: boolean) => {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ width, collapsed }));
  } catch {
    return;
  }
};

export const useSidebarLayoutStore = create<SidebarLayoutState>()((set, get) => ({
  ...readStored(),

  setCollapsed: (collapsed) => {
    set({ collapsed });
    get().persist();
  },

  toggle: () => get().setCollapsed(!get().collapsed),

  dragTo: (width) => {
    if (width < SIDEBAR_SNAP_WIDTH) set({ collapsed: true });
    else set({ collapsed: false, width: clampWidth(width) });
  },

  nudge: (delta) => {
    const { collapsed, width } = get();
    if (collapsed) {
      if (delta > 0) set({ collapsed: false, width: SIDEBAR_MIN_WIDTH });
    } else if (delta < 0 && width <= SIDEBAR_MIN_WIDTH) {
      set({ collapsed: true });
    } else {
      set({ width: clampWidth(width + delta) });
    }
    get().persist();
  },

  persist: () => {
    const { width, collapsed } = get();
    writeStored(width, collapsed);
  },
}));
