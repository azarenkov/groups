const KEY = "rift.voiceSession.v1";

export const RECONNECT_WINDOW_MS = 60_000;

export type VoiceSessionTarget =
  | { kind: "call"; callId: string }
  | { kind: "channel"; communityId: string; channelId: string };

interface StoredSession {
  userId: string;
  target: VoiceSessionTarget;
  at: number;
}

export const sameTarget = (a: VoiceSessionTarget, b: VoiceSessionTarget): boolean =>
  a.kind === "call" && b.kind === "call"
    ? a.callId === b.callId
    : a.kind === "channel" && b.kind === "channel" && a.communityId === b.communityId && a.channelId === b.channelId;

export function isFreshSession(stored: StoredSession | null, userId: string, now: number): stored is StoredSession {
  return !!stored && stored.userId === userId && now >= stored.at && now - stored.at <= RECONNECT_WINDOW_MS;
}

const read = (): StoredSession | null => {
  try {
    const stored = JSON.parse(sessionStorage.getItem(KEY) ?? "null") as StoredSession | null;
    return stored && typeof stored.userId === "string" && typeof stored.at === "number" && stored.target ? stored : null;
  } catch {
    return null;
  }
};

const write = (stored: StoredSession): void => {
  try {
    sessionStorage.setItem(KEY, JSON.stringify(stored));
  } catch {}
};

export function rememberVoiceSession(userId: string, target: VoiceSessionTarget, now = Date.now()): void {
  write({ userId, target, at: now });
}

export function touchVoiceSession(now = Date.now()): void {
  const stored = read();
  if (stored) write({ ...stored, at: now });
}

export function forgetVoiceSession(target?: VoiceSessionTarget): void {
  const stored = read();
  if (!stored || (target && !sameTarget(stored.target, target))) return;
  try {
    sessionStorage.removeItem(KEY);
  } catch {}
}

export function freshVoiceSession(userId: string, now = Date.now()): VoiceSessionTarget | null {
  const stored = read();
  if (isFreshSession(stored, userId, now)) return stored.target;
  if (stored) forgetVoiceSession();
  return null;
}

let touchInstalled = false;

export function installVoiceSessionTouch(): void {
  if (touchInstalled || typeof window === "undefined") return;
  touchInstalled = true;
  window.addEventListener("pagehide", () => touchVoiceSession());
}
