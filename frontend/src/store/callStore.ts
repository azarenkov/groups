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

  setIncoming: (call: IncomingCall) => void;
  clearIncoming: () => void;
  muteRingtone: () => void;
  participantJoined: (callId: string, userId: string, userName: string) => void;
  participantLeft: (callId: string, userId: string) => void;

  rehydrate: () => Promise<void>;
  start: (recipientIds: string[]) => Promise<void>;
  accept: () => Promise<void>;
  decline: () => Promise<void>;
  leave: () => Promise<void>;
  addParticipants: (recipientIds: string[]) => Promise<void>;
}

export const useCallStore = create<CallState>()((set, get) => ({
  incoming: null,
  activeCallId: null,
  connecting: false,
  ringtoneMuted: false,
  participants: {},

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
        ? { participants: { ...s.participants, [userId]: userName } }
        : s,
    ),
  participantLeft: (callId, userId) =>
    set((s) => {
      if (s.activeCallId !== callId) return s;
      const next = { ...s.participants };
      delete next[userId];
      return { participants: next };
    }),

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
        set({ activeCallId: null, participants: {} });
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
    set({ activeCallId: active.callId, participants });
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
    set({ activeCallId: null, participants: {} });
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
    await callAPI.addParticipants(callId, recipientIds);
  },
}));
