import { create } from "zustand";

import { userAPI } from "../../social/api/user";
import { useAuthStore } from "../../auth/store/authStore";
import { defaultMentionLevel } from "../../settings/store/notificationSettingsStore";

interface UnreadState {
  byChannel: Record<string, number>;
  byDm: Record<string, number>;
  channelToCommunity: Record<string, string>;
  setAll: (
    channels: Record<string, number>,
    dms: Record<string, number>,
    channelToCommunity?: Record<string, string>,
  ) => void;
  bumpChannel: (channelId: string, communityId?: string) => void;
  bumpDm: (friendId: string) => void;
  resetChannel: (channelId: string) => void;
  resetDm: (friendId: string) => void;
  clearAll: () => void;
  communityUnread: (communityId: string) => number;
}

let mutationGen = 0;

export const useUnreadStore = create<UnreadState>()((set, get) => ({
  byChannel: {},
  byDm: {},
  channelToCommunity: {},

  setAll: (channels, dms, channelToCommunity) =>
    set({
      byChannel: channels,
      byDm: dms,
      channelToCommunity: channelToCommunity ?? {},
    }),

  bumpChannel: (channelId, communityId) => {
    mutationGen += 1;
    set((state) => ({
      byChannel: {
        ...state.byChannel,
        [channelId]: (state.byChannel[channelId] ?? 0) + 1,
      },
      channelToCommunity: communityId
        ? { ...state.channelToCommunity, [channelId]: communityId }
        : state.channelToCommunity,
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
      const hasMapping = channelId in state.channelToCommunity;
      if (!hasUnread && !hasMapping) return state;
      const { [channelId]: _gone, ...rest } = state.byChannel;
      const { [channelId]: _g2, ...restMap } = state.channelToCommunity;
      return { byChannel: rest, channelToCommunity: restMap };
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
    set({ byChannel: {}, byDm: {}, channelToCommunity: {} });
  },

  communityUnread: (communityId) => {
    const { byChannel, channelToCommunity } = get();
    let total = 0;
    for (const [channelId, count] of Object.entries(byChannel)) {
      if (channelToCommunity[channelId] === communityId) total += count;
    }
    return total;
  },
}));

let refetchSeq = 0;
let refetchRetryTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleRefetchRetry(token: number) {
  if (token !== refetchSeq || refetchRetryTimer) return;
  refetchRetryTimer = setTimeout(() => {
    refetchRetryTimer = null;
    void refetchUnread().catch(() => {});
  }, 2000);
}

export async function refetchUnread(): Promise<void> {
  const sessionUserId = useAuthStore.getState().user?.id;
  if (!sessionUserId) return;
  const token = ++refetchSeq;
  for (let attempt = 0; attempt < 3; attempt++) {
    const gen = mutationGen;
    let u;
    try {
      u = await userAPI.getUnread(defaultMentionLevel());
    } catch (e) {
      scheduleRefetchRetry(token);
      throw e;
    }
    if (token !== refetchSeq) return;
    if (useAuthStore.getState().user?.id !== sessionUserId) return;
    if (mutationGen !== gen) continue;
    const channels: Record<string, number> = {};
    const channelToCommunity: Record<string, string> = {};
    for (const c of u.channels) {
      channels[c.channelId] = c.count;
      channelToCommunity[c.channelId] = c.communityId;
    }
    const dms: Record<string, number> = {};
    for (const d of u.dms) dms[d.friendId] = d.count;
    useUnreadStore.getState().setAll(channels, dms, channelToCommunity);
    return;
  }
  scheduleRefetchRetry(token);
}
