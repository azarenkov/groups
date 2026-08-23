import { create } from "zustand";

import { userAPI } from "../api/user";
import { useAuthStore } from "./authStore";

interface UnreadState {
  byChannel: Record<string, number>;
  byDm: Record<string, number>;
  channelToServer: Record<string, string>;
  setAll: (
    channels: Record<string, number>,
    dms: Record<string, number>,
    channelToServer?: Record<string, string>,
  ) => void;
  bumpChannel: (channelId: string, serverId?: string) => void;
  bumpDm: (friendId: string) => void;
  resetChannel: (channelId: string) => void;
  resetDm: (friendId: string) => void;
  clearAll: () => void;
  serverUnread: (serverId: string) => number;
}

let mutationGen = 0;

export const useUnreadStore = create<UnreadState>()((set, get) => ({
  byChannel: {},
  byDm: {},
  channelToServer: {},

  setAll: (channels, dms, channelToServer) =>
    set({
      byChannel: channels,
      byDm: dms,
      channelToServer: channelToServer ?? {},
    }),

  bumpChannel: (channelId, serverId) => {
    mutationGen += 1;
    set((state) => ({
      byChannel: {
        ...state.byChannel,
        [channelId]: (state.byChannel[channelId] ?? 0) + 1,
      },
      channelToServer: serverId
        ? { ...state.channelToServer, [channelId]: serverId }
        : state.channelToServer,
    }));
  },

  bumpDm: (friendId) => {
    mutationGen += 1;
    set((state) => ({
      byDm: { ...state.byDm, [friendId]: (state.byDm[friendId] ?? 0) + 1 },
    }));
  },

  resetChannel: (channelId) => {
    mutationGen += 1;
    set((state) => {
      const hasUnread = channelId in state.byChannel;
      const hasMapping = channelId in state.channelToServer;
      if (!hasUnread && !hasMapping) return state;
      const { [channelId]: _gone, ...rest } = state.byChannel;
      const { [channelId]: _g2, ...restMap } = state.channelToServer;
      return { byChannel: rest, channelToServer: restMap };
    });
  },

  resetDm: (friendId) => {
    mutationGen += 1;
    set((state) => {
      if (!(friendId in state.byDm)) return state;
      const { [friendId]: _gone, ...rest } = state.byDm;
      return { byDm: rest };
    });
  },

  clearAll: () => {
    mutationGen += 1;
    set({ byChannel: {}, byDm: {}, channelToServer: {} });
  },

  serverUnread: (serverId) => {
    const { byChannel, channelToServer } = get();
    let total = 0;
    for (const [channelId, count] of Object.entries(byChannel)) {
      if (channelToServer[channelId] === serverId) total += count;
    }
    return total;
  },
}));

let refetchSeq = 0;
let refetchRetryTimer: ReturnType<typeof setTimeout> | null = null;

export async function refetchUnread(): Promise<void> {
  const sessionUserId = useAuthStore.getState().user?.id;
  if (!sessionUserId) return;
  const token = ++refetchSeq;
  for (let attempt = 0; attempt < 3; attempt++) {
    const gen = mutationGen;
    let u;
    try {
      u = await userAPI.getUnread();
    } catch (e) {
      if (token === refetchSeq && !refetchRetryTimer) {
        refetchRetryTimer = setTimeout(() => {
          refetchRetryTimer = null;
          void refetchUnread().catch(() => {});
        }, 2000);
      }
      throw e;
    }
    if (token !== refetchSeq) return;
    if (useAuthStore.getState().user?.id !== sessionUserId) return;
    if (mutationGen !== gen) continue;
    const channels: Record<string, number> = {};
    const channelToServer: Record<string, string> = {};
    for (const c of u.channels) {
      channels[c.channelId] = c.count;
      channelToServer[c.channelId] = c.serverId;
    }
    const dms: Record<string, number> = {};
    for (const d of u.dms) dms[d.friendId] = d.count;
    useUnreadStore.getState().setAll(channels, dms, channelToServer);
    return;
  }
  if (token === refetchSeq && !refetchRetryTimer) {
    refetchRetryTimer = setTimeout(() => {
      refetchRetryTimer = null;
      void refetchUnread().catch(() => {});
    }, 2000);
  }
}
