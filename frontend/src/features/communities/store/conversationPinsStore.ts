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
  toggle: (kind: ConversationPinKind, id: string, keep?: Set<string>) => Promise<void>;
  applyRemote: (lists: Partial<PinLists>) => void;
  clearAll: () => void;
}

const api = {
  async list(): Promise<PinLists> {
    const response = await http.get("/v1/users/me/pins");
    return response.data.data;
  },
  async replace(kind: ConversationPinKind, ids: string[]): Promise<PinLists> {
    const response = await http.put(`/v1/users/me/pins/${kind}`, { ids });
    return response.data.data;
  },
};

const ids = (value: unknown): string[] | undefined =>
  Array.isArray(value) && value.every((v) => typeof v === "string") ? value : undefined;

let sessionGen = 0;
let pendingWrites = 0;
let remoteDuringWrite = false;
let latestFetch = 0;
let stateEpoch = 0;
let writeChain: Promise<unknown> = Promise.resolve();

export const togglePinned = (list: string[], id: string): string[] =>
  list.includes(id) ? list.filter((x) => x !== id) : [id, ...list];

export function pinnedFirst<T extends { id: string }>(items: T[], pinned: string[]): T[] {
  if (pinned.length === 0) return items;
  const rank = new Map(pinned.map((id, i) => [id, i]));
  const head = items.filter((item) => rank.has(item.id)).sort((a, b) => rank.get(a.id)! - rank.get(b.id)!);
  if (head.length === 0) return items;
  return [...head, ...items.filter((item) => !rank.has(item.id))];
}

export const useConversationPinsStore = create<ConversationPinsState>((set, get) => {
  const install = (lists: Partial<PinLists>) => {
    stateEpoch += 1;
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

    fetch: async () => {
      const gen = sessionGen;
      const request = ++latestFetch;
      const epoch = stateEpoch;
      const lists = await api.list();
      if (gen !== sessionGen || request !== latestFetch || epoch !== stateEpoch || pendingWrites > 0) return;
      install(lists);
    },

    toggle: (kind, id, keep) => {
      const gen = sessionGen;
      const before = keep ? get()[kind].filter((x) => keep.has(x)) : get()[kind];
      const next = togglePinned(before, id);
      if (next.length > MAX_PINNED_CONVERSATIONS) {
        return Promise.reject(new Error("PIN_LIMIT_REACHED"));
      }
      stateEpoch += 1;
      set({ [kind]: next } as Partial<PinLists>);
      pendingWrites += 1;
      const write = writeChain.then(() => api.replace(kind, get()[kind]));
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
            void get().fetch().catch(() => {});
          }
        });
    },

    applyRemote: (lists) => {
      if (pendingWrites > 0) {
        remoteDuringWrite = true;
        return;
      }
      install(lists);
    },

    clearAll: () => {
      sessionGen += 1;
      pendingWrites = 0;
      remoteDuringWrite = false;
      latestFetch += 1;
      stateEpoch += 1;
      writeChain = Promise.resolve();
      set({ dms: [], communities: [], loaded: false });
    },
  };
});
