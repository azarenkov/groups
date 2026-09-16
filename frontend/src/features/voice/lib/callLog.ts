import { callAPI } from "../api/call";
import { ApiError } from "../../../shared/lib/apiError";

interface CallLog {
  callId: string;
  startedAt: number;
  answeredAt: number | null;
  peerId: string;
  peerJoined: boolean;
  declined: boolean;
}

export interface PreparedCallLog {
  callId: string;
  peerId: string;
  status: string;
  duration: number | undefined;
}

let pendingLog: CallLog | null = null;

export function startCallLog(callId: string, peerId: string): void {
  pendingLog = {
    callId,
    startedAt: Date.now(),
    answeredAt: null,
    peerId,
    peerJoined: false,
    declined: false,
  };
}

export function clearCallLog(callId?: string): void {
  if (callId !== undefined && pendingLog?.callId !== callId) return;
  pendingLog = null;
}

export function markCallLogPeerJoined(userId: string): void {
  if (!pendingLog || pendingLog.peerId !== userId) return;
  pendingLog.declined = false;
  if (!pendingLog.peerJoined) {
    pendingLog.peerJoined = true;
    pendingLog.answeredAt = Date.now();
  }
}

export function markCallLogDeclined(userId: string): void {
  if (!pendingLog || pendingLog.peerId !== userId) return;
  pendingLog.declined = true;
}

export function takeCallLog(): PreparedCallLog | null {
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

export function dispatchCallLog(log: PreparedCallLog | null): void {
  if (!log) return;
  sendCallLog(log.callId, log.peerId, log.status, log.duration, 0);
}

export function finalizeCallLog(): void {
  dispatchCallLog(takeCallLog());
}

function sendCallLog(
  callId: string,
  peerId: string,
  status: string,
  duration: number | undefined,
  attempt: number,
): void {
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
