import { create } from "zustand";

export interface VoicePresenceMember {
  id: string;
  name: string;
  isMuted: boolean;
  mutedBy?: string;
  deafenedBy?: string;
  cameraOn: boolean;
  screenOn: boolean;
}

interface PendingOverlay {
  isMuted?: boolean;
  mutedBy?: string;
  deafenedBy?: string | null;
  communityId?: string;
}

const applyOverlay = (
  member: VoicePresenceMember,
  overlay: PendingOverlay,
): VoicePresenceMember => ({
  ...member,
  ...(overlay.isMuted !== undefined
    ? { isMuted: overlay.isMuted, mutedBy: overlay.mutedBy }
    : {}),
  ...(overlay.deafenedBy !== undefined
    ? { deafenedBy: overlay.deafenedBy ?? undefined }
    : {}),
});

interface VoicePresenceState {
  byChannel: Record<string, VoicePresenceMember[]>;
  pendingMutes: Record<string, Record<string, PendingOverlay>>;
  addMember: (channelId: string, member: VoicePresenceMember) => void;
  removeMember: (channelId: string, userId: string) => void;
  renameMember: (userId: string, name: string) => void;
  setMembers: (channelId: string, members: VoicePresenceMember[]) => void;
  setChannelMute: (
    channelId: string,
    userId: string,
    isMuted: boolean,
    mutedBy?: string,
    communityId?: string,
  ) => void;
  setChannelDeafen: (
    channelId: string,
    userId: string,
    deafenedBy: string | undefined,
    communityId?: string,
  ) => void;
  setChannelMedia: (
    channelId: string,
    userId: string,
    source: "camera" | "screen",
    enabled: boolean,
  ) => void;
  clearChannel: (channelId: string) => void;
  purgePendingMutes: (channelIds: string[], userId: string, communityId?: string) => void;
  purgePendingForCommunity: (communityId: string, channelIds: string[]) => void;
  resetRosters: () => void;
  clearAll: () => void;
}

export const useVoicePresenceStore = create<VoicePresenceState>()((set) => ({
  byChannel: {},
  pendingMutes: {},

  addMember: (channelId, incoming) =>
    set((state) => {
      const existing = state.byChannel[channelId] || [];
      const overlay = state.pendingMutes[channelId]?.[incoming.id];
      const member = overlay ? applyOverlay(incoming, overlay) : incoming;
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
                deafenedBy: member.deafenedBy ?? m.deafenedBy,
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
            overlay[m.id] ? applyOverlay(m, overlay[m.id]) : m,
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

  setChannelMute: (channelId, userId, isMuted, mutedBy, communityId) =>
    set((state) => {
      const existing = state.byChannel[channelId];
      if (!existing || !existing.some((m) => m.id === userId)) {
        return {
          pendingMutes: {
            ...state.pendingMutes,
            [channelId]: {
              ...(state.pendingMutes[channelId] ?? {}),
              [userId]: {
                ...(state.pendingMutes[channelId]?.[userId] ?? {}),
                isMuted,
                mutedBy: isMuted ? mutedBy : undefined,
                communityId,
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

  setChannelDeafen: (channelId, userId, deafenedBy, communityId) =>
    set((state) => {
      const existing = state.byChannel[channelId];
      if (!existing || !existing.some((m) => m.id === userId)) {
        return {
          pendingMutes: {
            ...state.pendingMutes,
            [channelId]: {
              ...(state.pendingMutes[channelId] ?? {}),
              [userId]: {
                ...(state.pendingMutes[channelId]?.[userId] ?? {}),
                deafenedBy: deafenedBy ?? null,
                communityId,
              },
            },
          },
        };
      }
      const next = existing.map((m) =>
        m.id === userId ? { ...m, deafenedBy } : m,
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

  purgePendingMutes: (channelIds, userId, communityId) =>
    set((state) => {
      let changed = false;
      const pendingMutes = { ...state.pendingMutes };
      const targets = new Set(channelIds);
      if (communityId) {
        for (const [channelId, overlay] of Object.entries(pendingMutes)) {
          if (overlay[userId]?.communityId === communityId) targets.add(channelId);
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

  purgePendingForCommunity: (communityId, channelIds) =>
    set((state) => {
      let changed = false;
      const pendingMutes = { ...state.pendingMutes };
      for (const [channelId, overlay] of Object.entries(pendingMutes)) {
        const belongs =
          channelIds.includes(channelId) ||
          Object.values(overlay).some((o) => o.communityId === communityId);
        if (!belongs) continue;
        delete pendingMutes[channelId];
        changed = true;
      }
      return changed ? { pendingMutes } : state;
    }),

  clearChannel: (channelId) =>
    set((state) => {
      const hasRoster = channelId in state.byChannel;
      const hasPending = channelId in state.pendingMutes;
      if (!hasRoster && !hasPending) return state;
      const { [channelId]: _gone, ...rest } = state.byChannel;
      const { [channelId]: _pendingGone, ...pendingRest } = state.pendingMutes;
      return { byChannel: rest, pendingMutes: pendingRest };
    }),

  resetRosters: () => set({ byChannel: {} }),

  clearAll: () => set({ byChannel: {}, pendingMutes: {} }),
}));
