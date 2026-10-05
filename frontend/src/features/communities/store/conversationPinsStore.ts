import { create } from "zustand";

import { http } from "../../../shared/api/http";

export type ConversationPinKind = "dms" | "communities";

export const MAX_PINNED_CONVERSATIONS = 10;

interface PinLists {
  dms: string[];
  communities: string[];
}

interface ConversationPinsState extends PinLists {
  loaded: boolean;
  fetch: () => Promise<void>;
  setPinned: (
    kind: ConversationPinKind,
    id: string,
    pinned: boolean,
    keep?: (pinnedId: string) => boolean,
  ) => Promise<void>;
  applyRemote: (lists: Partial<PinLists>) => void;
  remoteChanged: () => void;
  ensureLoaded: () => void;
  forget: (kind: ConversationPinKind, id: string) => void;
  clearAll: () => void;
}

const api = {
  async list(signal: AbortSignal): Promise<PinLists> {
    const response = await http.get("/v1/users/me/pins", { signal });
    return response.data.data;
  },
  async replace(kind: ConversationPinKind, ids: string[], signal: AbortSignal): Promise<PinLists> {
    const response = await http.put(`/v1/users/me/pins/${kind}`, { ids }, { signal });
    return response.data.data;
  },
};

const ids = (value: unknown): string[] | undefined =>
  Array.isArray(value) && value.every((v) => typeof v === "string") ? value : undefined;

let sessionGen = 0;
let pendingWrites = 0;
let remoteDuringWrite = false;
let latestFetch = 0;
let appliedFetch = 0;
let sessionAbort = new AbortController();
let stateEpoch = 0;
let writeChain: Promise<unknown> = Promise.resolve();
let initialLoad: Promise<void> | null = null;
const inflightFetches = new Set<Promise<void>>();
let ensureLoop = false;
let reconcileRunning = false;
let reconcileAgain = false;

export const PINS_LOAD_RETRY_MS = 1000;
export const PINS_LOAD_MAX_DELAY_MS = 30000;

export const withPinned = (list: string[], id: string, pinned: boolean): string[] =>
  pinned ? (list.includes(id) ? list : [id, ...list]) : list.filter((x) => x !== id);

export function pinnedFirst<T extends { id: string }>(items: T[], pinned: string[]): T[] {
  if (pinned.length === 0) return items;
  const rank = new Map(pinned.map((id, i) => [id, i]));
  const head = items.filter((item) => rank.has(item.id)).sort((a, b) => rank.get(a.id)! - rank.get(b.id)!);
  if (head.length === 0) return items;
  return [...head, ...items.filter((item) => !rank.has(item.id))];
}

export const useConversationPinsStore = create<ConversationPinsState>((set, get) => {
  const reconcile = () => {
    if (reconcileRunning) {
      reconcileAgain = true;
      return;
    }
    reconcileRunning = true;
    const gen = sessionGen;
    void (async () => {
      let delay = PINS_LOAD_RETRY_MS;
      while (gen === sessionGen) {
        reconcileAgain = false;
        try {
          await get().fetch();
          if (!reconcileAgain) break;
          continue;
        } catch (err) {
          console.warn("[pins] reload failed, retrying:", err);
        }
        if (gen !== sessionGen) break;
        const wait = delay;
        await new Promise((resolve) => setTimeout(resolve, wait));
        delay = Math.min(delay * 2, PINS_LOAD_MAX_DELAY_MS);
      }
      if (gen === sessionGen) reconcileRunning = false;
    })();
  };

  const install = (lists: Partial<PinLists>, fromFetch = false) => {
    if (!fromFetch) stateEpoch += 1;
    const dms = ids(lists.dms);
    const communities = ids(lists.communities);
    set({
      ...(dms ? { dms } : {}),
      ...(communities ? { communities } : {}),
      loaded: true,
    });
  };

  return {
    dms: [],
    communities: [],
    loaded: false,

    fetch: () => {
      const gen = sessionGen;
      const request = ++latestFetch;
      const epoch = stateEpoch;
      const promise = api.list(sessionAbort.signal).then((lists) => {
        if (gen !== sessionGen || request < appliedFetch || epoch !== stateEpoch || pendingWrites > 0) return;
        appliedFetch = request;
        install(lists, true);
      });
      inflightFetches.add(promise);
      void promise.then(
        () => inflightFetches.delete(promise),
        () => inflightFetches.delete(promise),
      );
      return promise;
    },

    setPinned: async (kind, id, pinned, keep) => {
      const gen = sessionGen;
      if (!get().loaded) {
        if (!initialLoad) {
          const load: Promise<void> = get()
            .fetch()
            .finally(() => {
              if (initialLoad === load) initialLoad = null;
            });
          initialLoad = load;
        }
        let loadError: unknown = null;
        await initialLoad.catch((err) => {
          loadError = err;
        });
        while (!get().loaded && inflightFetches.size > 0 && gen === sessionGen) {
          await Promise.allSettled([...inflightFetches]);
        }
        if (gen !== sessionGen) throw new Error("SESSION_CHANGED");
        if (!get().loaded) throw loadError ?? new Error("PINS_NOT_LOADED");
      }
      while (inflightFetches.size > 0) {
        await Promise.allSettled([...inflightFetches]);
        if (gen !== sessionGen) throw new Error("SESSION_CHANGED");
      }
      const before = keep ? get()[kind].filter((x) => x === id || keep(x)) : get()[kind];
      const next = withPinned(before, id, pinned);
      const current = get()[kind];
      if (next.length === current.length && next.every((x, i) => x === current[i])) return;
      if (next.length > MAX_PINNED_CONVERSATIONS) {
        throw new Error("PIN_LIMIT_REACHED");
      }
      stateEpoch += 1;
      set({ [kind]: next } as Partial<PinLists>);
      pendingWrites += 1;
      const write = writeChain.then(() => {
        if (gen !== sessionGen) throw new Error("SESSION_CHANGED");
        return api.replace(kind, get()[kind], sessionAbort.signal);
      });
      writeChain = write.catch(() => undefined);
      let failed = false;
      return write
        .then((lists) => {
          if (gen === sessionGen && pendingWrites === 1 && !remoteDuringWrite) install(lists);
        })
        .catch((err) => {
          failed = true;
          throw err;
        })
        .finally(() => {
          if (gen !== sessionGen) return;
          pendingWrites -= 1;
          if (pendingWrites > 0) return;
          if (remoteDuringWrite || failed) {
            remoteDuringWrite = false;
            reconcile();
          }
        });
    },

    ensureLoaded: () => {
      if (get().loaded || ensureLoop) return;
      ensureLoop = true;
      const gen = sessionGen;
      void (async () => {
        let delay = PINS_LOAD_RETRY_MS;
        while (gen === sessionGen && !get().loaded) {
          try {
            await get().fetch();
          } catch (err) {
            console.warn("[pins] initial load failed:", err);
          }
          if (gen !== sessionGen || get().loaded) break;
          const wait = delay;
          await new Promise((resolve) => setTimeout(resolve, wait));
          delay = Math.min(delay * 2, PINS_LOAD_MAX_DELAY_MS);
        }
        if (gen === sessionGen) ensureLoop = false;
      })();
    },

    remoteChanged: () => {
      if (pendingWrites > 0) {
        remoteDuringWrite = true;
        return;
      }
      reconcile();
    },

    applyRemote: (lists) => {
      if (pendingWrites > 0) {
        remoteDuringWrite = true;
        return;
      }
      install(lists);
    },

    forget: (kind, id) => {
      if (pendingWrites > 0) remoteDuringWrite = true;
      const hadFetches = inflightFetches.size > 0;
      stateEpoch += 1;
      if (get()[kind].includes(id)) {
        set({ [kind]: get()[kind].filter((x) => x !== id) } as Partial<PinLists>);
      }
      if (hadFetches && pendingWrites === 0) {
        reconcile();
      }
    },

    clearAll: () => {
      sessionGen += 1;
      pendingWrites = 0;
      remoteDuringWrite = false;
      latestFetch += 1;
      appliedFetch = latestFetch;
      sessionAbort.abort();
      sessionAbort = new AbortController();
      initialLoad = null;
      inflightFetches.clear();
      ensureLoop = false;
      reconcileRunning = false;
      reconcileAgain = false;
      stateEpoch += 1;
      writeChain = Promise.resolve();
      set({ dms: [], communities: [], loaded: false });
    },
  };
});
