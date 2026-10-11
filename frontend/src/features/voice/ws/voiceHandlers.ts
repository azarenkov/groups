import type { WsHandlers } from "../../../shared/ws/types";
import { useAuthStore } from "../../auth/store/authStore";
import { isBlockedEitherWay } from "../../social/store/blockStore";
import { useCallStore } from "../store/callStore";
import { useReadReceiptStore } from "../../chat/store/readReceiptStore";
import { useStreamViewersStore } from "../store/streamViewersStore";
import { useVoicePresenceStore } from "../store/voicePresenceStore";
import { useVoiceSessionStore } from "../store/voiceSessionStore";
import { asString } from "../../../shared/ws/context";
import { hiddenDmsKnown, hiddenDmsSettled, isDmHidden, useHiddenDmStore } from "../../dm/store/hiddenDmStore";

interface HeldCall {
  callId: string;
  callerId: string;
  callerName: string;
  video: boolean;
  selfId: string | undefined;
}

const heldCalls = new Map<string, HeldCall>();
let stopHolding: (() => void) | null = null;

const offerCall = (call: HeldCall) => {
  if (call.selfId !== useAuthStore.getState().user?.id) return;
  if (isBlockedEitherWay(call.callerId) || isDmHidden(call.callerId)) return;
  const { callId, callerId, callerName, video } = call;
  useCallStore.getState().setIncoming({ callId, callerId, callerName, video });
};

const holdUntilHiddenKnown = (call: HeldCall) => {
  heldCalls.set(call.callId, call);
  if (stopHolding) return;
  stopHolding = useHiddenDmStore.subscribe((s) => {
    if (!hiddenDmsSettled(s)) return;
    stopHolding?.();
    stopHolding = null;
    const calls = Array.from(heldCalls.values());
    heldCalls.clear();
    calls.forEach(offerCall);
  });
};

const endSessionIfEmpty = (channelId: string) => {
  if ((useVoicePresenceStore.getState().byChannel[channelId]?.length ?? 0) === 0) {
    useVoiceSessionStore.getState().end(channelId);
  }
};

export const voiceHandlers: WsHandlers = {
  "channel.participant.joined": (d) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const userName = asString(d.userName) ?? userId;
      const occurredAt = asString(d.occurredAt);
      if (occurredAt && (useVoicePresenceStore.getState().byChannel[channelId]?.length ?? 0) === 0) {
        useVoiceSessionStore.getState().begin(channelId, occurredAt);
      }
      useVoicePresenceStore.getState().addMember(channelId, {
        id: userId,
        name: userName,
        isMuted: false,
        cameraOn: false,
        screenOn: false,
      });
      return;
  },
  "channel.participant.left": (d) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      useVoicePresenceStore.getState().removeMember(channelId, userId);
      endSessionIfEmpty(channelId);
      return;
  },
  "channel.participant.connectionAborted": (d) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      useVoicePresenceStore.getState().removeMember(channelId, userId);
      endSessionIfEmpty(channelId);
      return;
  },
  "channel.text.joined": (d) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      const selfId = useAuthStore.getState().user?.id;
      if (channelId && userId && userId !== selfId) {
        useReadReceiptStore.getState().addChannelMember(channelId, userId);
      }
      return;
  },
  "call.incoming": (d) => {
      const callId = asString(d.callId);
      const callerId = asString(d.callerId);
      if (!callId || !callerId) return;
      if (isBlockedEitherWay(callerId)) return;
      const call: HeldCall = {
        callId,
        callerId,
        callerName: asString(d.callerName) ?? callerId,
        video: d.video === true,
        selfId: useAuthStore.getState().user?.id,
      };
      if (hiddenDmsKnown()) offerCall(call);
      else holdUntilHiddenKnown(call);
      return;
  },
  "call.participant.joined": (d) => {
      const callId = asString(d.callId);
      const userId = asString(d.userId);
      if (!callId || !userId) return;
      const userName = asString(d.userName) ?? userId;
      useCallStore.getState().participantJoined(callId, userId, userName);
      return;
  },
  "call.participant.left": (d) => {
      const callId = asString(d.callId);
      const userId = asString(d.userId);
      if (!callId || !userId) return;
      useCallStore.getState().participantLeft(callId, userId);
      return;
  },
  "call.declined": (d) => {
      const callId = asString(d.callId);
      const userId = asString(d.userId);
      if (callId && userId) {
        useCallStore.getState().participantDeclined(callId, userId);
      }
      return;
  },
  "call.available": (d) => {
      const callId = asString(d.callId);
      const userId = asString(d.userId);
      if (callId && userId && !isBlockedEitherWay(userId)) {
        useCallStore.getState().callAvailable(userId, callId);
      }
      return;
  },
  "call.ended": (d) => {
      const callId = asString(d.callId);
      if (!callId) return;
      heldCalls.delete(callId);
      useCallStore.getState().callEnded(callId);
      return;
  },
  "channel.participant.muted": (d, env) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const isMuted = env.type === "channel.participant.muted";
      useVoicePresenceStore
        .getState()
        .setChannelMute(
          channelId,
          userId,
          isMuted,
          isMuted ? (asString(d.mutedBy) ?? undefined) : undefined,
          asString(d.communityId) ?? undefined,
        );
      const selfId = useAuthStore.getState().user?.id;
      const mutedBy = asString(d.mutedBy) ?? asString(d.unmutedBy);
      if (userId === selfId && mutedBy && mutedBy !== selfId) {
        window.dispatchEvent(
          new CustomEvent(
            isMuted ? "rift:moderator-mute" : "rift:moderator-unmute",
            { detail: { channelId, mutedBy } },
          ),
        );
      }
      return;
  },
  "channel.participant.unmuted": (d, env) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const isMuted = env.type === "channel.participant.muted";
      useVoicePresenceStore
        .getState()
        .setChannelMute(
          channelId,
          userId,
          isMuted,
          isMuted ? (asString(d.mutedBy) ?? undefined) : undefined,
          asString(d.communityId) ?? undefined,
        );
      const selfId = useAuthStore.getState().user?.id;
      const mutedBy = asString(d.mutedBy) ?? asString(d.unmutedBy);
      if (userId === selfId && mutedBy && mutedBy !== selfId) {
        window.dispatchEvent(
          new CustomEvent(
            isMuted ? "rift:moderator-mute" : "rift:moderator-unmute",
            { detail: { channelId, mutedBy } },
          ),
        );
      }
      return;
  },
  "channel.participant.deafened": (d, env) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const isDeafened = env.type === "channel.participant.deafened";
      const deafenedBy = isDeafened ? (asString(d.deafenedBy) ?? undefined) : undefined;
      useVoicePresenceStore
        .getState()
        .setChannelDeafen(channelId, userId, deafenedBy, asString(d.communityId) ?? undefined);
      const selfId = useAuthStore.getState().user?.id;
      const actor = asString(d.deafenedBy) ?? asString(d.undeafenedBy);
      if (userId === selfId && actor && actor !== selfId) {
        window.dispatchEvent(
          new CustomEvent(
            isDeafened ? "rift:moderator-deafen" : "rift:moderator-undeafen",
            { detail: { channelId, deafenedBy: actor } },
          ),
        );
      }
      return;
  },
  "channel.participant.undeafened": (d, env) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const isDeafened = env.type === "channel.participant.deafened";
      const deafenedBy = isDeafened ? (asString(d.deafenedBy) ?? undefined) : undefined;
      useVoicePresenceStore
        .getState()
        .setChannelDeafen(channelId, userId, deafenedBy, asString(d.communityId) ?? undefined);
      const selfId = useAuthStore.getState().user?.id;
      const actor = asString(d.deafenedBy) ?? asString(d.undeafenedBy);
      if (userId === selfId && actor && actor !== selfId) {
        window.dispatchEvent(
          new CustomEvent(
            isDeafened ? "rift:moderator-deafen" : "rift:moderator-undeafen",
            { detail: { channelId, deafenedBy: actor } },
          ),
        );
      }
      return;
  },
  "channel.participant.camera.on": (d, env) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const source = env.type.includes("camera") ? "camera" : "screen";
      const enabled = env.type.endsWith(".on");
      useVoicePresenceStore
        .getState()
        .setChannelMedia(channelId, userId, source, enabled);
      return;
  },
  "channel.participant.camera.off": (d, env) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const source = env.type.includes("camera") ? "camera" : "screen";
      const enabled = env.type.endsWith(".on");
      useVoicePresenceStore
        .getState()
        .setChannelMedia(channelId, userId, source, enabled);
      return;
  },
  "channel.participant.screen.on": (d, env) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const source = env.type.includes("camera") ? "camera" : "screen";
      const enabled = env.type.endsWith(".on");
      useVoicePresenceStore
        .getState()
        .setChannelMedia(channelId, userId, source, enabled);
      return;
  },
  "channel.participant.screen.off": (d, env) => {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const source = env.type.includes("camera") ? "camera" : "screen";
      const enabled = env.type.endsWith(".on");
      useVoicePresenceStore
        .getState()
        .setChannelMedia(channelId, userId, source, enabled);
      return;
  },
  "channel.stream.viewers": (d) => {
      const communityId = asString(d.communityId);
      const channelId = asString(d.channelId);
      const streamerId = asString(d.streamerId);
      if (!communityId || !channelId || !streamerId) return;
      const count = typeof d.count === "number" ? d.count : 0;
      useStreamViewersStore
        .getState()
        .setCount(communityId, channelId, streamerId, count);
      return;
  },
};
