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

const getAuthedUserId = async (): Promise<string | null> => {
  const { useAuthStore } = await import("./authStore");
  return useAuthStore.getState().user?.id ?? null;
};

const getAuthedIdentity = async (): Promise<{
  userId: string | null;
  accessToken: string | null;
}> => {
  const { useAuthStore } = await import("./authStore");
  const state = useAuthStore.getState();
  return {
    userId: state.user?.id ?? null,
    accessToken: state.tokens?.accessToken ?? null,
  };
};

const leaveAsInitiator = async (
  callId: string,
  accessToken: string | null,
): Promise<void> => {
  if (accessToken) {
    await callAPI.leaveWithToken(callId, accessToken).catch(() => {});
  } else {
    await callAPI.leave(callId).catch(() => {});
  }
};

const awaitVoiceSettingsSyncedFor = async (
  userId: string | null,
): Promise<boolean> => {
  const { awaitVoiceSettingsSyncedFor: waitFor } = await import(
    "./voiceSettingsStore"
  );
  return waitFor(userId);
};

const armFallbackMuteRelease = (
  userId: string | null,
  callId: string,
  muteSeqAtFallback: number,
) => {
  if (!userId) return;
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
  video: boolean;
}

export interface StartCallOptions {
  video?: boolean;
}

const enableCameraOrFallback = async (callId: string): Promise<void> => {
  const service = getLiveKitService();
  if (useCallStore.getState().activeCallId !== callId || service.getActiveRoomName() !== callId) {
    return;
  }
  try {
    await service.setCameraEnabled(true);
  } catch (e) {
    console.warn("[call] camera unavailable, continuing audio-only:", e);
    if (useCallStore.getState().activeCallId === callId) {
      useCallStore
        .getState()
        .setCallError(i18n.t("cameraUnavailable", { ns: "call" }));
    }
  }
};

interface CallState {
  incoming: IncomingCall | null;
  activeCallId: string | null;
  connecting: boolean;
  participants: Record<string, string>;
  ringing: string[];
  focused: boolean;
  startedAt: number | null;
  attention: boolean;
  peerId: string | null;
  micMuted: boolean;
  micMuteSeq: number;
  joinableByPeer: Record<string, string>;
  callError: string | null;

  setIncoming: (call: IncomingCall) => void;
  clearIncoming: () => void;
  setFocused: (focused: boolean) => void;
  flagAttention: () => void;
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
  start: (recipientIds: string[], options?: StartCallOptions) => Promise<void>;
  accept: (withVideo?: boolean) => Promise<void>;
  decline: () => Promise<void>;
  dismissIncoming: () => void;
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
  activeCallId: null, startedAt: null, attention: false,
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
  setFocused: (focused) => set((s) => ({ focused, attention: focused ? false : s.attention })),
  flagAttention: () => set((s) => (s.focused || !s.activeCallId || s.attention ? s : { attention: true })),
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
        attention: s.focused ? s.attention : true,
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
      return {
        participants: next,
        ringing: without(s.ringing, userId),
        attention: s.focused ? s.attention : true,
      };
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
      set({ activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [], focused: false, peerId: null, micMuted: false });
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
        set({ activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [] });
      }
      return;
    }
    const participants = Object.fromEntries(
      active.participants.map((p) => [p.userId, p.userName]),
    );
    set((s) => ({
      activeCallId: active.callId,
      startedAt: s.activeCallId === active.callId ? s.startedAt : Date.now(),
      attention: s.activeCallId === active.callId ? s.attention : false,
      participants:
        s.activeCallId === active.callId
          ? { ...participants, ...s.participants }
          : participants,
      ringing: s.activeCallId === active.callId ? s.ringing : [],
      focused: s.activeCallId === active.callId ? s.focused : true,
      ...(settingsSynced ? {} : { micMuted: true }),
    }));
    const muteSeqBeforeConnect = get().micMuteSeq;
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
        if (get().activeCallId === active.callId) {
          set({ activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [], focused: false, peerId: null });
        }
        return;
      }
    }
    if ((useAuthStore.getState().user?.id ?? null) !== userId) {
      const svc = getLiveKitService();
      if (svc.getActiveRoomName() === active.callId) {
        await svc.disconnect().catch(() => {});
      }
      if (get().activeCallId === active.callId) {
        set({ activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [], focused: false, peerId: null });
      }
      return;
    }
    if (!settingsSynced && userId) {
      const muteSeqAtFallback = muteSeqBeforeConnect;
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
      const initiator = await getAuthedIdentity();
      const initiatorId = initiator.userId;
      const { token, url, participants } = await callAPI.join(callId);
      set({
        activeCallId: callId,
        startedAt: Date.now(),
        attention: false,
        participants: Object.fromEntries(
          participants.map((p) => [p.userId, p.userName]),
        ),
        ringing: [],
        focused: true,
        peerId: peerId ?? null,
        micMuted: false,
      });
      const muteSeqBeforeWait = get().micMuteSeq;
      const synced = await awaitVoiceSettingsSyncedFor(initiatorId);
      if (get().activeCallId !== callId) {
        await callAPI.leave(callId).catch(() => {});
        if (get().activeCallId === null) set({ connecting: false });
        return;
      }
      if ((await getAuthedUserId()) !== initiatorId) {
        set({
          activeCallId: null, startedAt: null, attention: false,
          connecting: false,
          participants: {},
          ringing: [],
          focused: false,
          peerId: null,
        });
        await leaveAsInitiator(callId, initiator.accessToken);
        return;
      }
      if (!synced) set({ micMuted: true });
      try {
        await connectToRoom(token, url, callId, !synced);
      } catch (e) {
        if (get().activeCallId === callId) {
          set({
            activeCallId: null, startedAt: null, attention: false,
            participants: {},
            ringing: [],
            focused: false,
            peerId: null,
          });
        }
        await callAPI.leave(callId).catch(() => {});
        throw e;
      }
      if (
        get().activeCallId !== callId ||
        (await getAuthedUserId()) !== initiatorId
      ) {
        const service = getLiveKitService();
        if (service.getActiveRoomName() === callId) {
          await service.disconnect().catch(() => {});
        }
        await leaveAsInitiator(callId, initiator.accessToken);
        if (get().activeCallId === callId) {
          set({
            activeCallId: null, startedAt: null, attention: false,
            connecting: false,
            participants: {},
            ringing: [],
            focused: false,
            peerId: null,
          });
        } else if (get().activeCallId === null) {
          set({ connecting: false });
        }
        return;
      }
      set({ connecting: false });
      if (!synced) armFallbackMuteRelease(initiatorId, callId, muteSeqBeforeWait);
    } catch (e) {
      set({ connecting: false });
      throw e;
    }
  },

  start: async (recipientIds, options) => {
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
    const initiator = await getAuthedIdentity().catch((e) => {
      set({ connecting: false });
      throw e;
    });
    const initiatorId = initiator.userId;
    const session = await callAPI.start(recipientIds, options?.video === true).catch((e) => {
      set({ connecting: false });
      throw e;
    });
    const { callId, token, url, participants } = session;
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
      startedAt: Date.now(),
      attention: false,
      participants: Object.fromEntries(
        participants.map((p) => [p.userId, p.userName]),
      ),
      ringing: recipientIds,
      focused: true,
      peerId: recipientIds[0] ?? null,
      micMuted: false,
    });
    const muteSeqBeforeWait = get().micMuteSeq;
    const synced = await awaitVoiceSettingsSyncedFor(initiatorId);
    if (get().activeCallId !== callId) {
      if (pendingLog?.callId === callId) pendingLog = null;
      await callAPI.leave(callId).catch(() => {});
      if (get().activeCallId === null) set({ connecting: false });
      return;
    }
    if ((await getAuthedUserId()) !== initiatorId) {
      pendingLog = null;
      set({
        activeCallId: null, startedAt: null, attention: false,
        connecting: false,
        participants: {},
        ringing: [],
        focused: false,
        peerId: null,
      });
      await leaveAsInitiator(callId, initiator.accessToken);
      return;
    }
    if (!synced) set({ micMuted: true });
    try {
      await connectToRoom(token, url, callId, !synced);
    } catch (e) {
      pendingLog = null;
      set({
        activeCallId: null, startedAt: null, attention: false,
        connecting: false,
        participants: {},
        ringing: [],
        focused: false,
        peerId: null,
      });
      await callAPI.leave(callId).catch(() => {});
      throw e;
    }
    if (
      get().activeCallId !== callId ||
      (await getAuthedUserId()) !== initiatorId
    ) {
      if (pendingLog?.callId === callId) pendingLog = null;
      const service = getLiveKitService();
      if (service.getActiveRoomName() === callId) {
        await service.disconnect().catch(() => {});
      }
      await leaveAsInitiator(callId, initiator.accessToken);
      if (get().activeCallId === callId) {
        set({
          activeCallId: null, startedAt: null, attention: false,
          connecting: false,
          participants: {},
          ringing: [],
          focused: false,
          peerId: null,
        });
      } else if (get().activeCallId === null) {
        set({ connecting: false });
      }
      return;
    }
    set({ connecting: false });
    if (!synced) armFallbackMuteRelease(initiatorId, callId, muteSeqBeforeWait);
    if (options?.video) await enableCameraOrFallback(callId);
  },

  accept: async (withVideo = false) => {
    const inc = get().incoming;
    if (!inc) return;
    if (get().activeCallId) await get().leave();
    set({ connecting: true, incoming: null });
    try {
      const initiator = await getAuthedIdentity();
      const initiatorId = initiator.userId;
      const { token, url, participants } = await callAPI.accept(inc.callId);
      set({
        activeCallId: inc.callId,
        startedAt: Date.now(),
        attention: false,
        participants: Object.fromEntries(
          participants.map((p) => [p.userId, p.userName]),
        ),
        ringing: [],
        focused: true,
        peerId: inc.callerId,
        micMuted: false,
      });
      const muteSeqBeforeWait = get().micMuteSeq;
      const synced = await awaitVoiceSettingsSyncedFor(initiatorId);
      if (get().activeCallId !== inc.callId) {
        await callAPI.leave(inc.callId).catch(() => {});
        if (get().activeCallId === null) set({ connecting: false });
        return;
      }
      if ((await getAuthedUserId()) !== initiatorId) {
        set({
          activeCallId: null, startedAt: null, attention: false,
          connecting: false,
          participants: {},
          ringing: [],
          focused: false,
          peerId: null,
        });
        await leaveAsInitiator(inc.callId, initiator.accessToken);
        return;
      }
      if (!synced) set({ micMuted: true });
      try {
        await connectToRoom(token, url, inc.callId, !synced);
      } catch (e) {
        if (get().activeCallId === inc.callId) {
          set({
            activeCallId: null, startedAt: null, attention: false,
            participants: {},
            ringing: [],
            focused: false,
            peerId: null,
          });
        }
        await callAPI.leave(inc.callId).catch(() => {});
        throw e;
      }
      if (
        get().activeCallId !== inc.callId ||
        (await getAuthedUserId()) !== initiatorId
      ) {
        const service = getLiveKitService();
        if (service.getActiveRoomName() === inc.callId) {
          await service.disconnect().catch(() => {});
        }
        await leaveAsInitiator(inc.callId, initiator.accessToken);
        if (get().activeCallId === inc.callId) {
          set({
            activeCallId: null, startedAt: null, attention: false,
            connecting: false,
            participants: {},
            ringing: [],
            focused: false,
            peerId: null,
          });
        } else if (get().activeCallId === null) {
          set({ connecting: false });
        }
        return;
      }
      set({ connecting: false });
      if (!synced) armFallbackMuteRelease(initiatorId, inc.callId, muteSeqBeforeWait);
      if (withVideo) await enableCameraOrFallback(inc.callId);
    } catch (e) {
      set({ connecting: false });
      throw e;
    }
  },

  dismissIncoming: () =>
    set((s) =>
      s.incoming
        ? {
            incoming: null,
            joinableByPeer: { ...s.joinableByPeer, [s.incoming.callerId]: s.incoming.callId },
          }
        : s,
    ),

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
    set({ activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [], focused: false, peerId: null, micMuted: false });
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
