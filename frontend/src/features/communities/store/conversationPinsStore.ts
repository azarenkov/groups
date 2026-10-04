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
  toggle: (kind: ConversationPinKind, id: string) => Promise<void>;
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

export const useConversationPinsStore = create<ConversationPinsState>((set, get) => ({
  dms: [],
  communities: [],
  loaded: false,

  fetch: async () => {
    const gen = sessionGen;
    const lists = await api.list();
    if (gen !== sessionGen || pendingWrites > 0) return;
    set({ dms: ids(lists.dms) ?? [], communities: ids(lists.communities) ?? [], loaded: true });
  },

  toggle: (kind, id) => {
    const gen = sessionGen;
    const before = get()[kind];
    const next = togglePinned(before, id);
    if (next.length > MAX_PINNED_CONVERSATIONS) {
      return Promise.reject(new Error("PIN_LIMIT_REACHED"));
    }
    set({ [kind]: next } as Partial<PinLists>);
    pendingWrites += 1;
    const write = writeChain.then(() => api.replace(kind, get()[kind]));
    writeChain = write.catch(() => undefined);
    return write
      .then((lists) => {
        if (gen !== sessionGen) return;
        if (pendingWrites === 1) {
          set({ dms: ids(lists.dms) ?? [], communities: ids(lists.communities) ?? [], loaded: true });
        }
      })
      .catch((err) => {
        if (gen === sessionGen && pendingWrites === 1) void get().fetch().catch(() => {});
        throw err;
      })
      .finally(() => {
        pendingWrites -= 1;
      });
  },

  applyRemote: (lists) => {
    if (pendingWrites > 0) return;
    const dms = ids(lists.dms);
    const communities = ids(lists.communities);
    set({
      ...(dms ? { dms } : {}),
      ...(communities ? { communities } : {}),
      loaded: true,
    });
  },

  clearAll: () => {
    sessionGen += 1;
    pendingWrites = 0;
    writeChain = Promise.resolve();
    set({ dms: [], communities: [], loaded: false });
  },
}));
