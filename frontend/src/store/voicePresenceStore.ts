import { create } from "zustand";

export interface VoicePresenceMember {
  id: string;
  name: string;
  isMuted: boolean;
  cameraOn: boolean;
  screenOn: boolean;
}

interface VoicePresenceState {
  byChannel: Record<string, VoicePresenceMember[]>;
  addMember: (channelId: string, member: VoicePresenceMember) => void;
  removeMember: (channelId: string, userId: string) => void;
  setMembers: (channelId: string, members: VoicePresenceMember[]) => void;
  setChannelMute: (
    channelId: string,
    userId: string,
    isMuted: boolean,
  ) => void;
  setChannelMedia: (
    channelId: string,
    userId: string,
    source: "camera" | "screen",
    enabled: boolean,
  ) => void;
  clearChannel: (channelId: string) => void;
  clearAll: () => void;
}

export const useVoicePresenceStore = create<VoicePresenceState>()((set) => ({
  byChannel: {},

  addMember: (channelId, member) =>
    set((state) => {
      const existing = state.byChannel[channelId] || [];
      if (existing.some((m) => m.id === member.id)) {
        const replaced = existing.map((m) =>
          m.id === member.id
            ? {
                ...m,
                name: member.name,
                isMuted: member.isMuted,
                cameraOn: member.cameraOn,
                screenOn: member.screenOn,
              }
            : m,
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

  setChannelMute: (channelId, userId, isMuted) =>
    set((state) => {
      const existing = state.byChannel[channelId];
      if (!existing) return state;
      const next = existing.map((m) =>
        m.id === userId ? { ...m, isMuted } : m,
      );
      return { byChannel: { ...state.byChannel, [channelId]: next } };
    }),

  setChannelMedia: (channelId, userId, source, enabled) =>
    set((state) => {
      const existing = state.byChannel[channelId];
      if (!existing) return state;
      const next = existing.map((m) =>
        m.id === userId
          ? source === "camera"
            ? { ...m, cameraOn: enabled }
            : { ...m, screenOn: enabled }
          : m,
      );
      return { byChannel: { ...state.byChannel, [channelId]: next } };
    }),

  clearChannel: (channelId) =>
    set((state) => {
      if (!(channelId in state.byChannel)) return state;
      const { [channelId]: _gone, ...rest } = state.byChannel;
      return { byChannel: rest };
    }),

  clearAll: () => set({ byChannel: {} }),
}));
