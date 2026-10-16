import { create } from "zustand";
import { callAPI } from "../api/call";
import { homeAPI } from "../../home/api/home";
import i18n from "../../../i18n";
import { getLiveKitService } from "../lib/livekit";
import { ApiError } from "../../../shared/lib/apiError";
import { declinedOwnInvite, type DeclineOwner } from "../lib/callCard";
import {
  awaitVoiceSettingsSyncedFor,
  connectToRoom,
  getAuthedIdentity,
  getAuthedUserId,
  leaveAsInitiator,
} from "../lib/callSession";
import { leaveVoiceChannel } from "./voiceChannelStore";
import { forgetVoiceSession, freshVoiceSession } from "../lib/sessionHint";
import { loadMediaState } from "../lib/mediaStateSync";

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

export interface EndedCall {
  callId: string;
  ringing: string[];
}

export interface CallNotice {
  kind: "declined" | "missed";
  userId: string;
  name: string | undefined;
}

interface CallState {
  incoming: IncomingCall | null;
  activeCallId: string | null;
  connecting: boolean;
  reconnecting: boolean;
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
  callNotice: CallNotice | null;
  endedCall: EndedCall | null;

  setIncoming: (call: IncomingCall) => void;
  clearIncoming: () => void;
  setFocused: (focused: boolean) => void;
  flagAttention: () => void;
  setMicMuted: (muted: boolean) => void;
  setDeafened: (deafened: boolean) => void;
  setCallError: (message: string | null) => void;
  setCallNotice: (notice: CallNotice | null) => void;
  callAvailable: (peerId: string, callId: string) => void;
  clearJoinable: (peerId: string) => void;
  participantJoined: (callId: string, userId: string, userName: string) => void;
  participantRenamed: (userId: string, userName: string) => void;
  participantLeft: (callId: string, userId: string) => void;
  participantDeclined: (callId: string, userId: string, userName: string | undefined, owner: DeclineOwner) => void;
  resetNotices: () => void;
  callEnded: (callId: string) => void;

  rehydrate: () => Promise<void>;
  joinCall: (callId: string, peerId?: string) => Promise<void>;
  start: (recipientIds: string[], options?: StartCallOptions) => Promise<void>;
  accept: (withVideo?: boolean) => Promise<void>;
  decline: () => Promise<void>;
  dismissIncoming: () => void;
  leave: (keepRoom?: boolean) => Promise<void>;
  callFriend: (friendId: string, video?: boolean) => void;
  addParticipants: (recipientIds: string[]) => Promise<void>;
}

const without = (list: string[], id: string) => list.filter((x) => x !== id);

interface EarlyCallEvents {
  declines: { userId: string; userName: string | undefined; owner: DeclineOwner }[];
  ended: boolean;
}

const earlyCallEvents = new Map<string, EarlyCallEvents>();

const earlyEventsFor = (callId: string): EarlyCallEvents => {
  const known = earlyCallEvents.get(callId);
  if (known) return known;
  const fresh: EarlyCallEvents = { declines: [], ended: false };
  earlyCallEvents.set(callId, fresh);
  return fresh;
};

export const useCallStore = create<CallState>()((set, get) => {
  const activate = (
    callId: string,
    participants: { userId: string; userName: string }[],
    ringing: string[],
    peerId: string | null,
  ) => {
    set({
      activeCallId: callId,
      endedCall: null,
      startedAt: Date.now(),
      attention: false,
      participants: Object.fromEntries(participants.map((p) => [p.userId, p.userName])),
      ringing,
      focused: true,
      peerId,
      micMuted: false,
      deafened: false,
      micMutedBeforeDeafen: false,
    });
    const early = earlyCallEvents.get(callId);
    earlyCallEvents.clear();
    for (const decline of early?.declines ?? []) get().participantDeclined(callId, decline.userId, decline.userName, decline.owner);
    if (early?.ended) get().callEnded(callId);
  };

  return {
  incoming: null,
  activeCallId: null, startedAt: null, attention: false,
  connecting: false,
  reconnecting: false,
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
  callNotice: null,
  endedCall: null,

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
  setCallNotice: (callNotice) => set({ callNotice }),
  resetNotices: () => {
    earlyCallEvents.clear();
    set({ callNotice: null, callError: null, endedCall: null });
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
  participantDeclined: (callId, userId, userName, owner) => {
    const s = get();
    const ours = s.activeCallId === callId || (!s.activeCallId && s.endedCall?.callId === callId);
    if (!ours) {
      if (s.connecting && !s.activeCallId) earlyEventsFor(callId).declines.push({ userId, userName, owner });
      return;
    }
    const active = s.activeCallId === callId;
    const ringing = active ? s.ringing : s.endedCall?.ringing ?? [];
    set({
      ...(active ? { ringing: without(ringing, userId) } : {}),
      ...(declinedOwnInvite(owner, userId, ringing) ? { callNotice: { kind: "declined" as const, userId, name: userName } } : {}),
    });
  },

  callEnded: (callId) => {
    const s = get();
    if (s.connecting && !s.activeCallId && s.incoming?.callId !== callId) earlyEventsFor(callId).ended = true;
    const joinable = Object.fromEntries(
      Object.entries(s.joinableByPeer).filter(([, cid]) => cid !== callId),
    );
    if (Object.keys(joinable).length !== Object.keys(s.joinableByPeer).length) {
      set({ joinableByPeer: joinable });
    }
    if (s.incoming?.callId === callId) set({ incoming: null });
    if (s.activeCallId === callId) {
      set({ endedCall: { callId, ringing: s.ringing }, activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [], focused: false, peerId: null, micMuted: false, deafened: false, micMutedBeforeDeafen: false });
      getLiveKitService()
        .disconnect()
        .catch(() => {});
    }
  },


  rehydrate: async () => {
    if (get().connecting) return;
    const { useAuthStore } = await import("../../auth/store/authStore");
    const userId = useAuthStore.getState().user?.id ?? null;
    const restoring = !get().activeCallId;
    const hint = restoring && userId ? freshVoiceSession(userId) : null;
    if (restoring && hint?.kind !== "call") return;
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
    const [active, media] = await Promise.all([
      callAPI.getActive().catch(() => null),
      restoring ? loadMediaState() : Promise.resolve(null),
    ]);
    if ((useAuthStore.getState().user?.id ?? null) !== userId) return;
    if (hint && (!active || hint.kind !== "call" || hint.callId !== active.callId)) {
      forgetVoiceSession(hint);
      return;
    }
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
      endedCall: s.activeCallId === active.callId ? s.endedCall : null,
      focused: s.activeCallId === active.callId ? s.focused : true,
      ...(media ? { micMuted: media.micMuted || !settingsSynced, deafened: false, micMutedBeforeDeafen: false } : {}),
      ...(settingsSynced ? {} : { micMuted: true }),
    }));
    const muteSeqBeforeConnect = get().micMuteSeq;
    const service = getLiveKitService();
    if (!service.isConnected() && !service.isConnectingNow()) {
      set({ reconnecting: true });
      try {
        await connectToRoom(
          active.token,
          active.url,
          active.callId,
          !settingsSynced || !!media?.micMuted,
        );
      } catch {
        if (get().activeCallId === active.callId) {
          set({ activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [], focused: false, peerId: null });
        }
        return;
      } finally {
        set({ reconnecting: false });
      }
      if (media?.deafened && get().activeCallId === active.callId) get().setDeafened(true);
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
    if (!settingsSynced && userId && !media?.micMuted) {
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
      activate(callId, participants, [], peerId ?? null);
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
    earlyCallEvents.clear();
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
    activate(callId, participants, recipientIds, recipientIds[0] ?? null);
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
    if (get().connecting) return;
    set({ connecting: true, incoming: null });
    if (get().activeCallId) await get().leave();
    await leaveVoiceChannel();
    try {
      const initiator = await getAuthedIdentity();
      const initiatorId = initiator.userId;
      const { token, url, participants } = await callAPI.accept(inc.callId);
      activate(inc.callId, participants, [], inc.callerId);
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
    set({ activeCallId: null, startedAt: null, attention: false, participants: {}, ringing: [], focused: false, peerId: null, micMuted: false, deafened: false, micMutedBeforeDeafen: false });
    if (!keepRoom) {
      await getLiveKitService()
        .disconnect()
        .catch(() => {});
    }
    if (callId) {
      await callAPI.leave(callId).catch(() => {});
    }
  },

  callFriend: (friendId, video = false) => {
    const cs = get();
    const inCallWithFriend =
      !!cs.activeCallId &&
      (cs.peerId === friendId || friendId in cs.participants || cs.ringing.includes(friendId));
    if (inCallWithFriend) {
      cs.setFocused(true);
      return;
    }
    cs.start([friendId], { video }).catch((e) => {
      const apiErr = e instanceof ApiError ? e : null;
      get().setCallError(
        apiErr?.code === "CALL_NOT_ALLOWED"
          ? i18n.t("callNotAllowed", { ns: "call" })
          : i18n.t("callFailed", { ns: "call" }),
      );
    });
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
  };
});
