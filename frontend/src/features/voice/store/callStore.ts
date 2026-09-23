import { create } from "zustand";
import { callAPI } from "../api/call";
import { homeAPI } from "../../home/api/home";
import i18n from "../../../i18n";
import { getLiveKitService } from "../lib/livekit";
import {
  clearCallLog,
  dispatchCallLog,
  finalizeCallLog,
  markCallLogDeclined,
  markCallLogPeerJoined,
  startCallLog,
  takeCallLog,
} from "../lib/callLog";
import {
  awaitVoiceSettingsSyncedFor,
  connectToRoom,
  getAuthedIdentity,
  getAuthedUserId,
  leaveAsInitiator,
} from "../lib/callSession";
import { leaveVoiceChannel } from "./voiceChannelStore";

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
      if (s.deafened || !s.micMuted) return;
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
  deafened: boolean;
  micMutedBeforeDeafen: boolean;
  joinableByPeer: Record<string, string>;
  callError: string | null;

  setIncoming: (call: IncomingCall) => void;
  clearIncoming: () => void;
  setFocused: (focused: boolean) => void;
  flagAttention: () => void;
  setMicMuted: (muted: boolean) => void;
  setDeafened: (deafened: boolean) => void;
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
  deafened: false,
  micMutedBeforeDeafen: false,
  joinableByPeer: {},
  callError: null,

  setIncoming: (call) =>
    set((s) => (s.activeCallId === call.callId ? s : { incoming: call })),
  clearIncoming: () => set({ incoming: null }),
  setFocused: (focused) => set((s) => ({ focused, attention: focused ? false : s.attention })),
  flagAttention: () => set((s) => (s.focused || !s.activeCallId || s.attention ? s : { attention: true })),
  setMicMuted: (muted) => {
    if (!muted && get().deafened) {
      set({ micMutedBeforeDeafen: false });
      get().setDeafened(false);
      if (!get().micMuted) return;
    }
    getLiveKitService().setMicrophoneMuted(muted);
    set((s) => ({ micMuted: muted, micMuteSeq: s.micMuteSeq + 1 }));
  },
  setDeafened: (deafened) => {
    const s = get();
    if (s.deafened === deafened) return;
    getLiveKitService().setDeafened(deafened);
    if (deafened) {
      set((state) => ({
        deafened: true,
        micMutedBeforeDeafen: state.micMuted,
        micMuteSeq: state.micMuteSeq + 1,
      }));
      if (!s.micMuted) s.setMicMuted(true);
      return;
    }
    const restoreMic = !s.micMutedBeforeDeafen && s.micMuted;
    set({ deafened: false, micMutedBeforeDeafen: false });
    if (restoreMic) s.setMicMuted(false);
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
      markCallLogPeerJoined(userId);
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
      markCallLogDeclined(userId);
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
      set({ activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [], focused: false, peerId: null, micMuted: false, deafened: false, micMutedBeforeDeafen: false });
      getLiveKitService()
        .disconnect()
        .catch(() => {});
    }
  },


  rehydrate: async () => {
    if (get().connecting) return;
    const { useAuthStore } = await import("../../auth/store/authStore");
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
    const recovered = active.startedAt ? Date.parse(active.startedAt) : NaN;
    const recoveredStartedAt = Number.isFinite(recovered)
      ? Math.min(recovered, Date.now())
      : Date.now();
    set((s) => ({
      activeCallId: active.callId,
      startedAt: s.activeCallId === active.callId ? s.startedAt : recoveredStartedAt,
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
          if (get().deafened || !get().micMuted) return;
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
        deafened: false,
        micMutedBeforeDeafen: false,
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
    startCallLog(callId, recipientIds[0]);
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
      deafened: false,
      micMutedBeforeDeafen: false,
    });
    const muteSeqBeforeWait = get().micMuteSeq;
    const synced = await awaitVoiceSettingsSyncedFor(initiatorId);
    if (get().activeCallId !== callId) {
      clearCallLog(callId);
      await callAPI.leave(callId).catch(() => {});
      if (get().activeCallId === null) set({ connecting: false });
      return;
    }
    if ((await getAuthedUserId()) !== initiatorId) {
      clearCallLog();
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
      clearCallLog();
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
      clearCallLog(callId);
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
    await leaveVoiceChannel();
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
        deafened: false,
        micMutedBeforeDeafen: false,
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
    set({ activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [], focused: false, peerId: null, micMuted: false, deafened: false, micMutedBeforeDeafen: false });
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
