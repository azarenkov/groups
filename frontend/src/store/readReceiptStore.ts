import { create } from "zustand";

type ChannelMemberReads = Record<string, string | null>;

interface ReadReceiptState {
  dmPeerReadAt: Record<string, string>;
  channelReads: Record<string, ChannelMemberReads>;
  setDmPeerReadAt: (friendId: string, readAt: string) => void;
  seedChannelReads: (
    channelId: string,
    members: { userId: string; readAt: string | null }[],
  ) => void;
  setChannelMemberRead: (
    channelId: string,
    memberId: string,
    readAt: string,
  ) => void;
  clear: () => void;
}

const newer = (prev: string | null | undefined, next: string): boolean =>
  !prev || new Date(next).getTime() > new Date(prev).getTime();

export const useReadReceiptStore = create<ReadReceiptState>()((set) => ({
  dmPeerReadAt: {},
  channelReads: {},

  setDmPeerReadAt: (friendId, readAt) =>
    set((s) => {
      if (!newer(s.dmPeerReadAt[friendId], readAt)) return s;
      return { dmPeerReadAt: { ...s.dmPeerReadAt, [friendId]: readAt } };
    }),

  seedChannelReads: (channelId, members) =>
    set((s) => {
      const map: ChannelMemberReads = {};
      for (const m of members) map[m.userId] = m.readAt;
      return { channelReads: { ...s.channelReads, [channelId]: map } };
    }),

  setChannelMemberRead: (channelId, memberId, readAt) =>
    set((s) => {
      const current = s.channelReads[channelId] ?? {};
      if (!newer(current[memberId], readAt)) return s;
      return {
        channelReads: {
          ...s.channelReads,
          [channelId]: { ...current, [memberId]: readAt },
        },
      };
    }),

  clear: () => set({ dmPeerReadAt: {}, channelReads: {} }),
}));
