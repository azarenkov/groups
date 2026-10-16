export type VoiceCue = "connected" | "disconnected" | "peerJoined" | "peerLeft";

export interface VoiceRoom {
  kind: "call" | "channel";
  id: string;
}

export interface VoiceCueState {
  room: VoiceRoom | null;
  settledAt: number;
  suspended: boolean;
}

export type VoiceCueEvent =
  | { type: "connected"; room: VoiceRoom; reconnect: boolean }
  | { type: "disconnected"; room: VoiceRoom; reconnecting: boolean }
  | { type: "suspended"; roomId: string | null }
  | { type: "resumed"; roomId: string | null }
  | { type: "peerJoined"; room: VoiceRoom; userId: string; selfId: string | null }
  | { type: "peerLeft"; room: VoiceRoom; userId: string; selfId: string | null };

export const SETTLE_MS = 1500;

export const idleVoiceCues: VoiceCueState = { room: null, settledAt: 0, suspended: false };

const sameRoom = (a: VoiceRoom | null, b: VoiceRoom): boolean => !!a && a.kind === b.kind && a.id === b.id;

export function nextVoiceCue(
  state: VoiceCueState,
  event: VoiceCueEvent,
  now: number,
): { state: VoiceCueState; cue: VoiceCue | null } {
  switch (event.type) {
    case "connected":
      return { state: { room: event.room, settledAt: now + SETTLE_MS, suspended: false }, cue: event.reconnect ? null : "connected" };
    case "disconnected":
      if (!sameRoom(state.room, event.room)) return { state, cue: null };
      return { state: idleVoiceCues, cue: event.reconnecting ? null : "disconnected" };
    case "suspended":
      if (!state.room || state.room.id !== event.roomId) return { state, cue: null };
      return { state: { ...state, suspended: true }, cue: null };
    case "resumed":
      if (!state.room || state.room.id !== event.roomId || !state.suspended) return { state, cue: null };
      return { state: { ...state, suspended: false, settledAt: now + SETTLE_MS }, cue: null };
    case "peerJoined":
    case "peerLeft": {
      const ours =
        sameRoom(state.room, event.room) && !state.suspended && event.userId !== event.selfId && now >= state.settledAt;
      return { state, cue: ours ? event.type : null };
    }
  }
}
