import { create } from "zustand";
import { callAPI } from "../api/call";
import { getLiveKitService } from "../services/livekit";

async function connectToRoom(
  token: string,
  url: string,
  roomName: string,
): Promise<void> {
  const service = getLiveKitService();
  if (service.isConnected() || service.isConnectingNow()) {
    await service.disconnect().catch(() => {});
  }
  await service.connect(token, url, roomName);
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
  participants: Record<string, string>;
  ringing: string[];
  focused: boolean;
  peerId: string | null;
  micMuted: boolean;
  joinableByPeer: Record<string, string>;

  setIncoming: (call: IncomingCall) => void;
  clearIncoming: () => void;
  setFocused: (focused: boolean) => void;
  setMicMuted: (muted: boolean) => void;
  callAvailable: (peerId: string, callId: string) => void;
  clearJoinable: (peerId: string) => void;
  participantJoined: (callId: string, userId: string, userName: string) => void;
  participantLeft: (callId: string, userId: string) => void;
  participantDeclined: (callId: string, userId: string) => void;
  callEnded: (callId: string) => void;

  rehydrate: () => Promise<void>;
  joinCall: (callId: string, peerId?: string) => Promise<void>;
  start: (recipientIds: string[]) => Promise<void>;
  accept: () => Promise<void>;
  decline: () => Promise<void>;
  leave: (keepRoom?: boolean) => Promise<void>;
  addParticipants: (recipientIds: string[]) => Promise<void>;
}

const without = (list: string[], id: string) => list.filter((x) => x !== id);

interface CallLog {
  startedAt: number;
  answeredAt: number | null;
  peerId: string;
  peerJoined: boolean;
  declined: boolean;
}

let pendingLog: CallLog | null = null;

function finalizeCallLog() {
  const cl = pendingLog;
  pendingLog = null;
  if (!cl) return;
  let status = "missed";
  let duration: number | undefined;
  if (cl.declined) {
    status = "declined";
  } else if (cl.peerJoined) {
    status = "completed";
    const from = cl.answeredAt ?? cl.startedAt;
    duration = Math.max(0, Math.round((Date.now() - from) / 1000));
  }
  callAPI.logCall(cl.peerId, status, duration).catch(() => {});
}

export const useCallStore = create<CallState>()((set, get) => ({
  incoming: null,
  activeCallId: null,
  connecting: false,
  participants: {},
  ringing: [],
  focused: false,
  peerId: null,
  micMuted: false,
  joinableByPeer: {},

  setIncoming: (call) =>
    set((s) => (s.activeCallId === call.callId ? s : { incoming: call })),
  clearIncoming: () => set({ incoming: null }),
  setFocused: (focused) => set({ focused }),
  setMicMuted: (muted) => {
    getLiveKitService().setMicrophoneMuted(muted);
    set({ micMuted: muted });
  },
  callAvailable: (peerId, callId) =>
    set((s) => ({ joinableByPeer: { ...s.joinableByPeer, [peerId]: callId } })),
  clearJoinable: (peerId) =>
    set((s) => {
      if (!(peerId in s.joinableByPeer)) return s;
      const next = { ...s.joinableByPeer };
      delete next[peerId];
      return { joinableByPeer: next };
    }),

  participantJoined: (callId, userId, userName) =>
    set((s) => {
      if (s.activeCallId !== callId) return s;
      if (pendingLog && pendingLog.peerId === userId && !pendingLog.peerJoined) {
        pendingLog.peerJoined = true;
        pendingLog.answeredAt = Date.now();
      }
      return {
        participants: { ...s.participants, [userId]: userName },
        ringing: without(s.ringing, userId),
      };
    }),
  participantLeft: (callId, userId) =>
    set((s) => {
      if (s.activeCallId !== callId) return s;
      const next = { ...s.participants };
      delete next[userId];
      return { participants: next, ringing: without(s.ringing, userId) };
    }),
  participantDeclined: (callId, userId) =>
    set((s) => {
      if (s.activeCallId !== callId) return s;
      if (pendingLog && pendingLog.peerId === userId) pendingLog.declined = true;
      return { ringing: without(s.ringing, userId) };
    }),

  callEnded: (callId) => {
    const s = get();
    const joinable = Object.fromEntries(
      Object.entries(s.joinableByPeer).filter(([, cid]) => cid !== callId),
    );
    if (Object.keys(joinable).length !== Object.keys(s.joinableByPeer).length) {
      set({ joinableByPeer: joinable });
    }
    if (s.incoming?.callId === callId) set({ incoming: null });
    if (s.activeCallId === callId) {
      finalizeCallLog();
      set({ activeCallId: null, participants: {}, ringing: [], focused: false, peerId: null, micMuted: false });
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
      await connectToRoom(active.token, active.url, active.callId).catch(
        () => {},
      );
    }
    set((s) => ({
      activeCallId: active.callId,
      participants,
      ringing: s.activeCallId === active.callId ? s.ringing : [],
      focused: s.activeCallId === active.callId ? s.focused : true,
    }));
  },

  joinCall: async (callId, peerId) => {
    if (get().activeCallId === callId) {
      set({ focused: true });
      return;
    }
    if (get().activeCallId) await get().leave();
    set({ connecting: true });
    try {
      const { token, url, participants } = await callAPI.join(callId);
      try {
        await connectToRoom(token, url, callId);
      } catch (e) {
        await callAPI.leave(callId).catch(() => {});
        throw e;
      }
      set({
        activeCallId: callId,
        connecting: false,
        participants: Object.fromEntries(
          participants.map((p) => [p.userId, p.userName]),
        ),
        ringing: [],
        focused: true,
        peerId: peerId ?? null,
        micMuted: false,
      });
    } catch (e) {
      set({ connecting: false });
      throw e;
    }
  },

  start: async (recipientIds) => {
    if (get().activeCallId) await get().leave();
    set({ connecting: true });
    const session = await callAPI.start(recipientIds).catch((e) => {
      set({ connecting: false });
      throw e;
    });
    const { callId, token, url, participants } = session;
    pendingLog = {
      startedAt: Date.now(),
      answeredAt: null,
      peerId: recipientIds[0],
      peerJoined: false,
      declined: false,
    };
    set({
      activeCallId: callId,
      participants: Object.fromEntries(
        participants.map((p) => [p.userId, p.userName]),
      ),
      ringing: recipientIds,
      focused: true,
      peerId: recipientIds[0] ?? null,
      micMuted: false,
    });
    try {
      await connectToRoom(token, url, callId);
    } catch (e) {
      pendingLog = null;
      set({
        activeCallId: null,
        connecting: false,
        participants: {},
        ringing: [],
        focused: false,
        peerId: null,
      });
      await callAPI.leave(callId).catch(() => {});
      throw e;
    }
    set({ connecting: false });
  },

  accept: async () => {
    const inc = get().incoming;
    if (!inc) return;
    if (get().activeCallId) await get().leave();
    set({ connecting: true, incoming: null });
    try {
      const { token, url, participants } = await callAPI.accept(inc.callId);
      try {
        await connectToRoom(token, url, inc.callId);
      } catch (e) {
        await callAPI.leave(inc.callId).catch(() => {});
        throw e;
      }
      set({
        activeCallId: inc.callId,
        connecting: false,
        participants: Object.fromEntries(
          participants.map((p) => [p.userId, p.userName]),
        ),
        ringing: [],
        focused: true,
        peerId: inc.callerId,
        micMuted: false,
      });
    } catch (e) {
      set({ connecting: false });
      throw e;
    }
  },

  decline: async () => {
    const inc = get().incoming;
    set((s) =>
      inc
        ? {
            incoming: null,
            joinableByPeer: { ...s.joinableByPeer, [inc.callerId]: inc.callId },
          }
        : { incoming: null },
    );
    if (inc) {
      await callAPI.decline(inc.callId).catch(() => {});
    }
  },

  leave: async (keepRoom = false) => {
    const callId = get().activeCallId;
    finalizeCallLog();
    set({ activeCallId: null, participants: {}, ringing: [], focused: false, peerId: null, micMuted: false });
    if (!keepRoom) {
      await getLiveKitService()
        .disconnect()
        .catch(() => {});
    }
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
    try {
      await callAPI.addParticipants(callId, recipientIds);
    } catch (e) {
      set((s) => ({
        ringing: s.ringing.filter((id) => !recipientIds.includes(id)),
      }));
      throw e;
    }
  },
}));
