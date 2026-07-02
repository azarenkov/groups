import { create } from "zustand";
import { callAPI } from "../api/call";
import { getLiveKitService } from "../services/livekit";

async function connectToRoom(token: string, url: string): Promise<void> {
  const service = getLiveKitService();
  if (service.isConnected() || service.isConnectingNow()) {
    await service.disconnect().catch(() => {});
  }
  await service.connect(token, url);
}

export interface IncomingCall {
  callId: string;
  callerId: string;
  callerName: string;
}

interface CallState {
  incoming: IncomingCall | null;
  activeCallId: string | null;
  connecting: boolean;
  ringtoneMuted: boolean;
  participants: Record<string, string>;
  ringing: string[];

  setIncoming: (call: IncomingCall) => void;
  clearIncoming: () => void;
  muteRingtone: () => void;
  participantJoined: (callId: string, userId: string, userName: string) => void;
  participantLeft: (callId: string, userId: string) => void;
  participantDeclined: (callId: string, userId: string) => void;
  callEnded: (callId: string) => void;

  rehydrate: () => Promise<void>;
  start: (recipientIds: string[]) => Promise<void>;
  accept: () => Promise<void>;
  decline: () => Promise<void>;
  leave: () => Promise<void>;
  addParticipants: (recipientIds: string[]) => Promise<void>;
}

const without = (list: string[], id: string) => list.filter((x) => x !== id);

export const useCallStore = create<CallState>()((set, get) => ({
  incoming: null,
  activeCallId: null,
  connecting: false,
  ringtoneMuted: false,
  participants: {},
  ringing: [],

  setIncoming: (call) =>
    set((s) =>
      s.activeCallId === call.callId
        ? s
        : { incoming: call, ringtoneMuted: false },
    ),
  clearIncoming: () => set({ incoming: null }),
  muteRingtone: () => set({ ringtoneMuted: true }),

  participantJoined: (callId, userId, userName) =>
    set((s) =>
      s.activeCallId === callId
        ? {
            participants: { ...s.participants, [userId]: userName },
            ringing: without(s.ringing, userId),
          }
        : s,
    ),
  participantLeft: (callId, userId) =>
    set((s) => {
      if (s.activeCallId !== callId) return s;
      const next = { ...s.participants };
      delete next[userId];
      return { participants: next, ringing: without(s.ringing, userId) };
    }),
  participantDeclined: (callId, userId) =>
    set((s) =>
      s.activeCallId === callId ? { ringing: without(s.ringing, userId) } : s,
    ),

  callEnded: (callId) => {
    const s = get();
    if (s.incoming?.callId === callId) set({ incoming: null });
    if (s.activeCallId === callId) {
      set({ activeCallId: null, participants: {}, ringing: [] });
      getLiveKitService()
        .disconnect()
        .catch(() => {});
    }
  },

  rehydrate: async () => {
    if (get().connecting) return;
    const active = await callAPI.getActive().catch(() => null);
    if (!active) {
      const service = getLiveKitService();
      if (
        get().activeCallId &&
        !service.isConnected() &&
        !service.isConnectingNow()
      ) {
        set({ activeCallId: null, participants: {}, ringing: [] });
      }
      return;
    }
    const participants = Object.fromEntries(
      active.participants.map((p) => [p.userId, p.userName]),
    );
    const service = getLiveKitService();
    if (!service.isConnected() && !service.isConnectingNow()) {
      await connectToRoom(active.token, active.url).catch(() => {});
    }
    set((s) => ({
      activeCallId: active.callId,
      participants,
      ringing: s.activeCallId === active.callId ? s.ringing : [],
    }));
  },

  start: async (recipientIds) => {
    set({ connecting: true });
    try {
      const { callId, token, url, participants } =
        await callAPI.start(recipientIds);
      await connectToRoom(token, url);
      set({
        activeCallId: callId,
        connecting: false,
        participants: Object.fromEntries(
          participants.map((p) => [p.userId, p.userName]),
        ),
        ringing: recipientIds,
      });
    } catch (e) {
      set({ connecting: false });
      throw e;
    }
  },

  accept: async () => {
    const inc = get().incoming;
    if (!inc) return;
    set({ connecting: true, incoming: null });
    try {
      const { token, url, participants } = await callAPI.accept(inc.callId);
      await connectToRoom(token, url);
      set({
        activeCallId: inc.callId,
        connecting: false,
        participants: Object.fromEntries(
          participants.map((p) => [p.userId, p.userName]),
        ),
        ringing: [],
      });
    } catch (e) {
      set({ connecting: false });
      throw e;
    }
  },

  decline: async () => {
    const inc = get().incoming;
    set({ incoming: null });
    if (inc) {
      await callAPI.decline(inc.callId).catch(() => {});
    }
  },

  leave: async () => {
    const callId = get().activeCallId;
    set({ activeCallId: null, participants: {}, ringing: [] });
    await getLiveKitService()
      .disconnect()
      .catch(() => {});
    if (callId) {
      await callAPI.leave(callId).catch(() => {});
    }
  },

  addParticipants: async (recipientIds) => {
    const callId = get().activeCallId;
    if (!callId) return;
    set((s) => ({
      ringing: Array.from(new Set([...s.ringing, ...recipientIds])),
    }));
    await callAPI.addParticipants(callId, recipientIds);
  },
}));
