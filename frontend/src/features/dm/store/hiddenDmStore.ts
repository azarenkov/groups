import { create } from "zustand";

import { http } from "../../../shared/api/http";

interface HiddenDmState {
  hidden: string[];
  loaded: boolean;
  fetch: () => Promise<void>;
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

export const withHidden = (list: string[], id: string, hidden: boolean): string[] =>
  hidden ? (list.includes(id) ? list : [...list, id]) : list.filter((x) => x !== id);

export const useHiddenDmStore = create<HiddenDmState>((set, get) => ({
  hidden: [],
  loaded: false,

  fetch: async () => {
    const gen = sessionGen;
    const startedAt = epoch;
    const peers = await api.list(sessionAbort.signal);
    if (gen !== sessionGen || startedAt !== epoch || pendingWrites > 0) return;
    set({ hidden: peers, loaded: true });
  },

  setHidden: async (peerId, hidden) => {
    const gen = sessionGen;
    epoch += 1;
    set({ hidden: withHidden(get().hidden, peerId, hidden) });
    pendingWrites += 1;
    try {
      await (hidden ? api.hide(peerId, sessionAbort.signal) : api.unhide(peerId, sessionAbort.signal));
    } catch (err) {
      if (gen === sessionGen) {
        pendingWrites -= 1;
        epoch += 1;
        void get()
          .fetch()
          .catch((e) => console.warn("[hiddenDms] reload failed:", e));
      }
      throw err;
    }
    if (gen === sessionGen) pendingWrites -= 1;
  },

  applyRemote: (peerId, hidden) => {
    epoch += 1;
    set({ hidden: withHidden(get().hidden, peerId, hidden) });
  },

  clearAll: () => {
    sessionGen += 1;
    epoch += 1;
    pendingWrites = 0;
    sessionAbort.abort();
    sessionAbort = new AbortController();
    set({ hidden: [], loaded: false });
  },
}));

export const isDmHidden = (peerId: string): boolean => useHiddenDmStore.getState().hidden.includes(peerId);
