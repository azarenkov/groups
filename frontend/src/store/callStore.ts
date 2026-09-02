import { create } from "zustand";
import { callAPI } from "../api/call";
import { homeAPI } from "../api/home";
import i18n from "../i18n";
import { ApiError } from "../utils/apiError";
import { getLiveKitService } from "../services/livekit";

async function connectToRoom(
  token: string,
  url: string,
  roomName: string,
  startMuted?: boolean,
): Promise<void> {
  const service = getLiveKitService();
  if (service.isConnected() || service.isConnectingNow()) {
    await service.disconnect().catch(() => {});
  }
  await service.connect(token, url, roomName, startMuted);
}

const awaitVoiceSettingsSynced = async (): Promise<{
  synced: boolean;
  userId: string | null;
}> => {
  const { useAuthStore } = await import("./authStore");
  const userId = useAuthStore.getState().user?.id ?? null;
  if (!userId) return { synced: true, userId };
  const { voiceSettingsReadyFor } = await import("./voiceSettingsStore");
  const synced = await Promise.race([
    voiceSettingsReadyFor(userId).then(() => true),
    new Promise<boolean>((resolve) => setTimeout(() => resolve(false), 4000)),
  ]);
  return { synced, userId };
};

const armFallbackMuteRelease = (userId: string | null, callId: string) => {
  if (!userId) return;
  const muteSeqAtFallback = useCallStore.getState().micMuteSeq;
  void import("./voiceSettingsStore").then(({ voiceSettingsReadyFor }) =>
    voiceSettingsReadyFor(userId).then(() => {
      const s = useCallStore.getState();
      if (s.activeCallId !== callId) return;
      if (s.micMuteSeq !== muteSeqAtFallback) return;
      if (!s.micMuted) return;
      s.setMicMuted(false);
    }),
  );
};

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
  micMuteSeq: number;
  joinableByPeer: Record<string, string>;
  callError: string | null;

  setIncoming: (call: IncomingCall) => void;
  clearIncoming: () => void;
  setFocused: (focused: boolean) => void;
  setMicMuted: (muted: boolean) => void;
  setCallError: (message: string | null) => void;
  callAvailable: (peerId: string, callId: string) => void;
  clearJoinable: (peerId: string) => void;
  participantJoined: (callId: string, userId: string, userName: string) => void;
  participantRenamed: (userId: string, userName: string) => void;
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
  callId: string;
  startedAt: number;
  answeredAt: number | null;
  peerId: string;
  peerJoined: boolean;
  declined: boolean;
}

let pendingLog: CallLog | null = null;

interface PreparedCallLog {
  callId: string;
  peerId: string;
  status: string;
  duration: number | undefined;
}

function takeCallLog(): PreparedCallLog | null {
  const cl = pendingLog;
  pendingLog = null;
  if (!cl) return null;
  let status = "missed";
  let duration: number | undefined;
  if (cl.declined) {
    status = "declined";
  } else if (cl.peerJoined) {
    status = "completed";
    const from = cl.answeredAt ?? cl.startedAt;
    duration = Math.max(0, Math.round((Date.now() - from) / 1000));
  }
  return { callId: cl.callId, peerId: cl.peerId, status, duration };
}

function dispatchCallLog(log: PreparedCallLog | null) {
  if (!log) return;
  sendCallLog(log.callId, log.peerId, log.status, log.duration, 0);
}

function finalizeCallLog() {
  dispatchCallLog(takeCallLog());
}

function sendCallLog(
  callId: string,
  peerId: string,
  status: string,
  duration: number | undefined,
  attempt: number,
) {
  callAPI.logCall(callId, peerId, status, duration).catch((e) => {
    const code = e instanceof ApiError ? e.code : null;
    const terminal =
      code === "CALL_ALREADY_LOGGED" ||
      code === "SELF_CALL" ||
      code === "INVALID_CALL_STATUS" ||
      code === "CALLS_CLOSED";
    if (attempt < 30 && !terminal) {
      window.setTimeout(
        () => sendCallLog(callId, peerId, status, duration, attempt + 1),
        Math.min(30000, 2000 * 2 ** attempt),
      );
    }
  });
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
  micMuteSeq: 0,
  joinableByPeer: {},
  callError: null,

  setIncoming: (call) =>
    set((s) => (s.activeCallId === call.callId ? s : { incoming: call })),
  clearIncoming: () => set({ incoming: null }),
  setFocused: (focused) => set({ focused }),
  setMicMuted: (muted) => {
    getLiveKitService().setMicrophoneMuted(muted);
    set((s) => ({ micMuted: muted, micMuteSeq: s.micMuteSeq + 1 }));
  },
  setCallError: (callError) => set({ callError }),
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
      if (pendingLog && pendingLog.peerId === userId) {
        pendingLog.declined = false;
        if (!pendingLog.peerJoined) {
          pendingLog.peerJoined = true;
          pendingLog.answeredAt = Date.now();
        }
      }
      return {
        participants: { ...s.participants, [userId]: userName },
        ringing: without(s.ringing, userId),
      };
    }),
  participantRenamed: (userId, userName) =>
    set((s) => {
      const incoming =
        s.incoming && s.incoming.callerId === userId
          ? { ...s.incoming, callerName: userName }
          : s.incoming;
      const participants =
        userId in s.participants
          ? { ...s.participants, [userId]: userName }
          : s.participants;
      if (incoming === s.incoming && participants === s.participants) return s;
      return { incoming, participants };
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
    const { useAuthStore } = await import("./authStore");
    const userId = useAuthStore.getState().user?.id ?? null;
    let settingsSynced = true;
    if (userId) {
      const { voiceSettingsReadyFor } = await import("./voiceSettingsStore");
      settingsSynced = await Promise.race([
        voiceSettingsReadyFor(userId).then(() => true),
        new Promise<boolean>((resolve) =>
          setTimeout(() => resolve(false), 4000),
        ),
      ]);
    }
    if ((useAuthStore.getState().user?.id ?? null) !== userId) return;
    const active = await callAPI.getActive().catch(() => null);
    if ((useAuthStore.getState().user?.id ?? null) !== userId) return;
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
      try {
        await connectToRoom(
          active.token,
          active.url,
          active.callId,
          !settingsSynced,
        );
      } catch {
        return;
      }
    }
    set((s) => ({
      activeCallId: active.callId,
      participants,
      ringing: s.activeCallId === active.callId ? s.ringing : [],
      focused: s.activeCallId === active.callId ? s.focused : true,
      ...(settingsSynced ? {} : { micMuted: true }),
    }));
    if (!settingsSynced && userId) {
      const muteSeqAtFallback = get().micMuteSeq;
      void import("./voiceSettingsStore").then(({ voiceSettingsReadyFor }) =>
        voiceSettingsReadyFor(userId).then(() => {
          if (get().activeCallId !== active.callId) return;
          if (get().micMuteSeq !== muteSeqAtFallback) return;
          if (!get().micMuted) return;
          get().setMicMuted(false);
        }),
      );
    }
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
      const settings = await awaitVoiceSettingsSynced();
      try {
        await connectToRoom(token, url, callId, !settings.synced);
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
        micMuted: !settings.synced,
      });
      if (!settings.synced) armFallbackMuteRelease(settings.userId, callId);
    } catch (e) {
      set({ connecting: false });
      throw e;
    }
  },

  start: async (recipientIds) => {
    if (get().activeCallId) {
      if (recipientIds.length === 1) {
        try {
          const target = await homeAPI.profile(recipientIds[0]);
          if (!target.acceptsCalls || !target.active) {
            get().setCallError(i18n.t("callNotAllowed", { ns: "call" }));
            return;
          }
        } catch {
          get().setCallError(i18n.t("callFailed", { ns: "call" }));
          return;
        }
      }
      await get().leave();
    }
    set({ connecting: true });
    const session = await callAPI.start(recipientIds).catch((e) => {
      set({ connecting: false });
      throw e;
    });
    const { callId, token, url, participants } = session;
    const settings = await awaitVoiceSettingsSynced();
    pendingLog = {
      callId,
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
      micMuted: !settings.synced,
    });
    try {
      await connectToRoom(token, url, callId, !settings.synced);
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
    if (!settings.synced) armFallbackMuteRelease(settings.userId, callId);
  },

  accept: async () => {
    const inc = get().incoming;
    if (!inc) return;
    if (get().activeCallId) await get().leave();
    set({ connecting: true, incoming: null });
    try {
      const { token, url, participants } = await callAPI.accept(inc.callId);
      const settings = await awaitVoiceSettingsSynced();
      try {
        await connectToRoom(token, url, inc.callId, !settings.synced);
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
      if (!settings.synced) {
        set({ micMuted: true });
        armFallbackMuteRelease(settings.userId, inc.callId);
      }
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
    const log = takeCallLog();
    set({ activeCallId: null, participants: {}, ringing: [], focused: false, peerId: null, micMuted: false });
    if (!keepRoom) {
      await getLiveKitService()
        .disconnect()
        .catch(() => {});
    }
    if (callId) {
      await callAPI.leave(callId).catch(() => {});
    }
    dispatchCallLog(log);
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
