import { DisconnectReason } from "livekit-client";

const RECOVERABLE = new Set<DisconnectReason>([
  DisconnectReason.UNKNOWN_REASON,
  DisconnectReason.SERVER_SHUTDOWN,
  DisconnectReason.STATE_MISMATCH,
  DisconnectReason.MIGRATION,
  DisconnectReason.SIGNAL_CLOSE,
  DisconnectReason.ROOM_CLOSED,
]);

export const isRecoverableDisconnect = (reason: DisconnectReason): boolean => RECOVERABLE.has(reason);
