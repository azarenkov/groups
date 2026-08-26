import { create } from "zustand";

export interface VoicePresenceMember {
  id: string;
  name: string;
  isMuted: boolean;
  mutedBy?: string;
  cameraOn: boolean;
  screenOn: boolean;
}

interface VoicePresenceState {
  byChannel: Record<string, VoicePresenceMember[]>;
  pendingMutes: Record<
    string,
    Record<string, { isMuted: boolean; mutedBy?: string; serverId?: string }>
  >;
  addMember: (channelId: string, member: VoicePresenceMember) => void;
  removeMember: (channelId: string, userId: string) => void;
  renameMember: (userId: string, name: string) => void;
  setMembers: (channelId: string, members: VoicePresenceMember[]) => void;
  setChannelMute: (
    channelId: string,
    userId: string,
    isMuted: boolean,
    mutedBy?: string,
    serverId?: string,
  ) => void;
  setChannelMedia: (
    channelId: string,
    userId: string,
    source: "camera" | "screen",
    enabled: boolean,
  ) => void;
  clearChannel: (channelId: string) => void;
  purgePendingMutes: (channelIds: string[], userId: string, serverId?: string) => void;
  purgePendingForServer: (serverId: string, channelIds: string[]) => void;
  clearAll: () => void;
}

export const useVoicePresenceStore = create<VoicePresenceState>()((set) => ({
  byChannel: {},
  pendingMutes: {},

  addMember: (channelId, incoming) =>
    set((state) => {
      const existing = state.byChannel[channelId] || [];
      const overlay = state.pendingMutes[channelId]?.[incoming.id];
      const member = overlay
        ? { ...incoming, isMuted: overlay.isMuted, mutedBy: overlay.mutedBy }
        : incoming;
      let pendingMutes = state.pendingMutes;
      if (overlay) {
        const channelPending = { ...state.pendingMutes[channelId] };
        delete channelPending[incoming.id];
        pendingMutes = { ...state.pendingMutes };
        if (Object.keys(channelPending).length === 0) {
          delete pendingMutes[channelId];
        } else {
          pendingMutes[channelId] = channelPending;
        }
      }
      if (existing.some((m) => m.id === member.id)) {
        const replaced = existing.map((m) =>
          m.id === member.id
            ? {
                ...m,
                name: member.name,
                isMuted: member.isMuted,
                mutedBy: member.mutedBy,
                cameraOn: member.cameraOn,
                screenOn: member.screenOn,
              }
            : m,
        );
        return {
          byChannel: { ...state.byChannel, [channelId]: replaced },
          pendingMutes,
        };
      }
      return {
        byChannel: { ...state.byChannel, [channelId]: [...existing, member] },
        pendingMutes,
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
    set((state) => {
      const overlay = state.pendingMutes[channelId];
      const merged = overlay
        ? members.map((m) =>
            overlay[m.id]
              ? { ...m, isMuted: overlay[m.id].isMuted, mutedBy: overlay[m.id].mutedBy }
              : m,
          )
        : members;
      const pendingMutes = { ...state.pendingMutes };
      if (overlay) {
        const remaining = { ...overlay };
        for (const m of members) delete remaining[m.id];
        if (Object.keys(remaining).length === 0) {
          delete pendingMutes[channelId];
        } else {
          pendingMutes[channelId] = remaining;
        }
      }
      return {
        byChannel: { ...state.byChannel, [channelId]: merged },
        pendingMutes,
      };
    }),

  renameMember: (userId, name) =>
    set((state) => {
      let changed = false;
      const byChannel: Record<string, VoicePresenceMember[]> = {};
      for (const [channelId, members] of Object.entries(state.byChannel)) {
        if (members.some((m) => m.id === userId)) {
          changed = true;
          byChannel[channelId] = members.map((m) =>
            m.id === userId ? { ...m, name } : m,
          );
        } else {
          byChannel[channelId] = members;
        }
      }
      return changed ? { byChannel } : state;
    }),

  setChannelMute: (channelId, userId, isMuted, mutedBy, serverId) =>
    set((state) => {
      const existing = state.byChannel[channelId];
      if (!existing || !existing.some((m) => m.id === userId)) {
        return {
          pendingMutes: {
            ...state.pendingMutes,
            [channelId]: {
              ...(state.pendingMutes[channelId] ?? {}),
              [userId]: {
                isMuted,
                mutedBy: isMuted ? mutedBy : undefined,
                serverId,
              },
            },
          },
        };
      }
      const next = existing.map((m) =>
        m.id === userId
          ? { ...m, isMuted, mutedBy: isMuted ? mutedBy : undefined }
          : m,
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

  purgePendingMutes: (channelIds, userId, serverId) =>
    set((state) => {
      let changed = false;
      const pendingMutes = { ...state.pendingMutes };
      const targets = new Set(channelIds);
      if (serverId) {
        for (const [channelId, overlay] of Object.entries(pendingMutes)) {
          if (overlay[userId]?.serverId === serverId) targets.add(channelId);
        }
      }
      for (const channelId of targets) {
        const overlay = pendingMutes[channelId];
        if (!overlay || !overlay[userId]) continue;
        const next = { ...overlay };
        delete next[userId];
        if (Object.keys(next).length === 0) delete pendingMutes[channelId];
        else pendingMutes[channelId] = next;
        changed = true;
      }
      return changed ? { pendingMutes } : state;
    }),

  purgePendingForServer: (serverId, channelIds) =>
    set((state) => {
      let changed = false;
      const pendingMutes = { ...state.pendingMutes };
      for (const [channelId, overlay] of Object.entries(pendingMutes)) {
        const belongs =
          channelIds.includes(channelId) ||
          Object.values(overlay).some((o) => o.serverId === serverId);
        if (!belongs) continue;
        delete pendingMutes[channelId];
        changed = true;
      }
      return changed ? { pendingMutes } : state;
    }),

  clearChannel: (channelId) =>
    set((state) => {
      if (!(channelId in state.byChannel)) return state;
      const { [channelId]: _gone, ...rest } = state.byChannel;
      return { byChannel: rest };
    }),

  clearAll: () => set({ byChannel: {}, pendingMutes: {} }),
}));
