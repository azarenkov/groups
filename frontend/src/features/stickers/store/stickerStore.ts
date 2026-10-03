import { create } from "zustand";

import { stickersAPI, type PackSticker, type StickerPack } from "../api/stickers";

const RECENT_LIMIT = 24;
const recentKey = (userId: string) => `rift.recentStickers.${userId}`;

const readRecent = (userId: string | null): PackSticker[] => {
  if (!userId) return [];
  try {
    const raw = localStorage.getItem(recentKey(userId));
    const parsed = raw ? (JSON.parse(raw) as PackSticker[]) : [];
    return Array.isArray(parsed) ? parsed.slice(0, RECENT_LIMIT) : [];
  } catch {
    return [];
  }
};

const writeRecent = (userId: string | null, recent: PackSticker[]) => {
  if (!userId) return;
  try {
    localStorage.setItem(recentKey(userId), JSON.stringify(recent));
  } catch {}
};

interface StickerState {
  userId: string | null;
  packs: StickerPack[];
  recent: PackSticker[];
  loaded: boolean;
  failed: boolean;
  load: (userId: string) => Promise<void>;
  refresh: () => Promise<void>;
  refreshIfStale: (maxAgeMs: number) => Promise<void>;
  upsertPack: (pack: StickerPack) => void;
  dropPack: (packId: string) => void;
  install: (pack: StickerPack) => Promise<void>;
  uninstall: (packId: string) => Promise<void>;
  rememberUsed: (sticker: PackSticker) => void;
  forgetSticker: (stickerId: string) => void;
  clear: () => void;
}

let generation = 0;
let revision = 0;
let refreshedAt = 0;

export const useStickerStore = create<StickerState>()((set, get) => ({
  userId: null,
  packs: [],
  recent: [],
  loaded: false,
  failed: false,

  load: async (userId) => {
    if (get().userId !== userId) {
      generation += 1;
      set({ userId, packs: [], recent: readRecent(userId), loaded: false, failed: false });
    }
    await get().refresh();
  },

  refresh: async () => {
    const gen = generation;
    const rev = revision;
    try {
      const packs = await stickersAPI.mine();
      if (gen !== generation) return;
      if (rev !== revision) return get().refresh();
      refreshedAt = Date.now();
      set({ packs, loaded: true, failed: false });
    } catch {
      if (gen === generation) set({ failed: true });
    }
  },

  refreshIfStale: async (maxAgeMs) => {
    if (Date.now() - refreshedAt < maxAgeMs) return;
    await get().refresh();
  },

  upsertPack: (pack) => {
    revision += 1;
    set((state) => {
      const existing = state.packs.find((p) => p.id === pack.id);
      const next = { ...pack, installed: pack.installed ?? existing?.installed };
      return {
        packs: existing ? state.packs.map((p) => (p.id === pack.id ? next : p)) : [...state.packs, next],
      };
    });
  },

  dropPack: (packId) => {
    revision += 1;
    const recent = get().recent.filter((s) => s.packId !== packId);
    set((state) => ({ packs: state.packs.filter((p) => p.id !== packId), recent }));
    writeRecent(get().userId, recent);
  },

  install: async (pack) => {
    const gen = generation;
    await stickersAPI.install(pack.id);
    const full = pack.stickers.length >= pack.stickerCount ? pack : await stickersAPI.pack(pack.id);
    if (gen !== generation) return;
    get().upsertPack({ ...full, installed: true });
  },

  uninstall: async (packId) => {
    const gen = generation;
    await stickersAPI.uninstall(packId);
    if (gen !== generation) return;
    get().dropPack(packId);
  },

  rememberUsed: (sticker) => {
    const recent = [sticker, ...get().recent.filter((s) => s.id !== sticker.id)].slice(0, RECENT_LIMIT);
    set({ recent });
    writeRecent(get().userId, recent);
  },

  forgetSticker: (stickerId) => {
    revision += 1;
    const recent = get().recent.filter((s) => s.id !== stickerId);
    set((state) => ({
      recent,
      packs: state.packs.map((p) =>
        p.stickers.some((s) => s.id === stickerId)
          ? {
              ...p,
              stickers: p.stickers.filter((s) => s.id !== stickerId),
              stickerCount: Math.max(0, p.stickerCount - 1),
            }
          : p,
      ),
    }));
    writeRecent(get().userId, recent);
  },

  clear: () => {
    generation += 1;
    refreshedAt = 0;
    set({ userId: null, packs: [], recent: [], loaded: false, failed: false });
  },
}));

export const stickerSession = (): (() => boolean) => {
  const gen = generation;
  return () => gen === generation;
};

export const packCover = (pack: StickerPack): PackSticker | undefined =>
  pack.stickers.find((s) => s.id === pack.coverStickerId) ?? pack.stickers[0];

export const moveSticker = (ids: string[], id: string, to: number): string[] => {
  const from = ids.indexOf(id);
  if (from < 0) return ids;
  const target = Math.max(0, Math.min(ids.length - 1, to));
  if (target === from) return ids;
  const next = ids.filter((x) => x !== id);
  next.splice(target, 0, id);
  return next;
};

export const isInstalled = (pack: StickerPack): boolean => pack.installed !== false;

export const installedPacks = (packs: StickerPack[]): StickerPack[] => packs.filter(isInstalled);

export const searchMyStickers = (packs: StickerPack[], emoji: string): PackSticker[] => {
  const needle = emoji.trim();
  if (!needle) return [];
  return packs.flatMap((pack) => pack.stickers.filter((s) => s.emoji.includes(needle)));
};
