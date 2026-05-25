import { create } from "zustand";

export interface VoicePresenceMember {
  id: string;
  name: string;
}

interface VoicePresenceState {
  byChannel: Record<string, VoicePresenceMember[]>;
  addMember: (channelId: string, member: VoicePresenceMember) => void;
  removeMember: (channelId: string, userId: string) => void;
  setMembers: (channelId: string, members: VoicePresenceMember[]) => void;
  clearChannel: (channelId: string) => void;
}

export const useVoicePresenceStore = create<VoicePresenceState>()((set) => ({
  byChannel: {},

  addMember: (channelId, member) =>
    set((state) => {
      const existing = state.byChannel[channelId] || [];
      if (existing.some((m) => m.id === member.id)) {
        const replaced = existing.map((m) =>
          m.id === member.id ? { ...m, name: member.name } : m,
        );
        return {
          byChannel: { ...state.byChannel, [channelId]: replaced },
        };
      }
      return {
        byChannel: { ...state.byChannel, [channelId]: [...existing, member] },
      };
    }),

  removeMember: (channelId, userId) =>
    set((state) => {
      const existing = state.byChannel[channelId];
      if (!existing) return state;
      const next = existing.filter((m) => m.id !== userId);
      if (next.length === 0) {
        const { [channelId]: _gone, ...rest } = state.byChannel;
        return { byChannel: rest };
      }
      return { byChannel: { ...state.byChannel, [channelId]: next } };
    }),

  setMembers: (channelId, members) =>
    set((state) => ({
      byChannel: { ...state.byChannel, [channelId]: members },
    })),

  clearChannel: (channelId) =>
    set((state) => {
      if (!(channelId in state.byChannel)) return state;
      const { [channelId]: _gone, ...rest } = state.byChannel;
      return { byChannel: rest };
    }),
}));
