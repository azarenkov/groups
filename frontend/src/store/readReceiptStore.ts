import { create } from "zustand";

type ChannelMemberReads = Record<string, string | null>;

interface ReadReceiptState {
  dmPeerReadAt: Record<string, string>;
  channelReads: Record<string, ChannelMemberReads>;
  setDmPeerReadAt: (friendId: string, readAt: string) => void;
  seedDmPeerReadAt: (friendId: string, readAt: string | null) => void;
  seedChannelReads: (
    channelId: string,
    members: { userId: string; readAt: string | null }[],
  ) => void;
  setChannelMemberRead: (
    channelId: string,
    memberId: string,
    readAt: string,
  ) => void;
  addChannelMember: (channelId: string, memberId: string) => void;
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

  seedDmPeerReadAt: (friendId, readAt) =>
    set((s) => {
      if (readAt == null) return s;
      if (!newer(s.dmPeerReadAt[friendId], readAt)) return s;
      return { dmPeerReadAt: { ...s.dmPeerReadAt, [friendId]: readAt } };
    }),

  seedChannelReads: (channelId, members) =>
    set((s) => {
      const existing = s.channelReads[channelId] ?? {};
      const map: ChannelMemberReads = {};
      for (const m of members) {
        const cur = existing[m.userId];
        map[m.userId] =
          m.readAt && newer(cur, m.readAt) ? m.readAt : (cur ?? m.readAt);
      }
      return { channelReads: { ...s.channelReads, [channelId]: map } };
    }),

  setChannelMemberRead: (channelId, memberId, readAt) =>
    set((s) => {
      const current = s.channelReads[channelId];
      if (!current) return s;
      if (!newer(current[memberId], readAt)) return s;
      return {
        channelReads: {
          ...s.channelReads,
          [channelId]: { ...current, [memberId]: readAt },
        },
      };
    }),

  addChannelMember: (channelId, memberId) =>
    set((s) => {
      const cur = s.channelReads[channelId];
      if (!cur || memberId in cur) return s;
      return {
        channelReads: {
          ...s.channelReads,
          [channelId]: { ...cur, [memberId]: null },
        },
      };
    }),

  clear: () => set({ dmPeerReadAt: {}, channelReads: {} }),
}));
