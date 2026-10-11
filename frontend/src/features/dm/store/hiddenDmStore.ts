import { create } from "zustand";

import { http } from "../../../shared/api/http";

interface HiddenDmState {
  hidden: string[];
  loaded: boolean;
  fetch: () => Promise<void>;
  ensureLoaded: () => void;
  markStale: () => void;
  setHidden: (peerId: string, hidden: boolean) => Promise<void>;
  applyRemote: (peerId: string, hidden: boolean) => void;
  clearAll: () => void;
}

const api = {
  async list(signal: AbortSignal): Promise<string[]> {
    const response = await http.get("/v1/users/me/hidden-dms", { signal });
    const peers = response.data?.data?.peers;
    return Array.isArray(peers) ? peers.filter((p: unknown): p is string => typeof p === "string") : [];
  },
  async hide(peerId: string, signal: AbortSignal): Promise<void> {
    await http.put(`/v1/users/me/hidden-dms/${peerId}`, undefined, { signal });
  },
  async unhide(peerId: string, signal: AbortSignal): Promise<void> {
    await http.delete(`/v1/users/me/hidden-dms/${peerId}`, { signal });
  },
};

let sessionGen = 0;
let epoch = 0;
let pendingWrites = 0;
let sessionAbort = new AbortController();
let needsReconcile = false;
let ensureLoop = false;
let writeChain: Promise<unknown> = Promise.resolve();

export const HIDDEN_LOAD_RETRY_MS = 1000;
export const HIDDEN_LOAD_MAX_DELAY_MS = 30000;

export const withHidden = (list: string[], id: string, hidden: boolean): string[] =>
  hidden ? (list.includes(id) ? list : [...list, id]) : list.filter((x) => x !== id);

export const useHiddenDmStore = create<HiddenDmState>((set, get) => {
  const settle = () => {
    if (pendingWrites > 0 || !needsReconcile) return;
    needsReconcile = false;
    if (!get().loaded) {
      get().ensureLoaded();
      return;
    }
    void get()
      .fetch()
      .catch((e) => console.warn("[hiddenDms] reload failed:", e));
  };

  return {
    hidden: [],
    loaded: false,

    fetch: async () => {
      const gen = sessionGen;
      for (;;) {
        const startedAt = epoch;
        const peers = await api.list(sessionAbort.signal);
        if (gen !== sessionGen) return;
        if (pendingWrites > 0) {
          needsReconcile = true;
          return;
        }
        if (startedAt !== epoch) continue;
        set({ hidden: peers, loaded: true });
        return;
      }
    },

    ensureLoaded: () => {
      if (get().loaded || ensureLoop) return;
      ensureLoop = true;
      const gen = sessionGen;
      void (async () => {
        let delay = HIDDEN_LOAD_RETRY_MS;
        while (gen === sessionGen && !get().loaded) {
          try {
            await get().fetch();
          } catch (err) {
            console.warn("[hiddenDms] load failed:", err);
          }
          if (gen !== sessionGen || get().loaded || pendingWrites > 0) break;
          const wait = delay;
          await new Promise((resolve) => setTimeout(resolve, wait));
          delay = Math.min(delay * 2, HIDDEN_LOAD_MAX_DELAY_MS);
        }
        if (gen === sessionGen) ensureLoop = false;
      })();
    },

    markStale: () => {
      set({ loaded: false });
      get().ensureLoaded();
    },

    setHidden: async (peerId, hidden) => {
      const gen = sessionGen;
      epoch += 1;
      set({ hidden: withHidden(get().hidden, peerId, hidden) });
      pendingWrites += 1;
      const signal = sessionAbort.signal;
      const write = writeChain.then(() => {
        if (gen !== sessionGen) throw new Error("SESSION_CHANGED");
        return hidden ? api.hide(peerId, signal) : api.unhide(peerId, signal);
      });
      writeChain = write.catch(() => undefined);
      try {
        await write;
      } catch (err) {
        if (gen === sessionGen) {
          pendingWrites -= 1;
          epoch += 1;
          needsReconcile = true;
          settle();
        }
        throw err;
      }
      if (gen === sessionGen) {
        pendingWrites -= 1;
        settle();
      }
    },

    applyRemote: (peerId, hidden) => {
      epoch += 1;
      set({ hidden: withHidden(get().hidden, peerId, hidden) });
    },

    clearAll: () => {
      sessionGen += 1;
      epoch += 1;
      pendingWrites = 0;
      needsReconcile = false;
      ensureLoop = false;
      writeChain = Promise.resolve();
      sessionAbort.abort();
      sessionAbort = new AbortController();
      set({ hidden: [], loaded: false });
    },
  };
});

export const isDmHidden = (peerId: string): boolean => useHiddenDmStore.getState().hidden.includes(peerId);

export const hiddenDmsKnown = (): boolean => useHiddenDmStore.getState().loaded;
