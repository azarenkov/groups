import React from "react";

export interface EmojiStatus {
  stickerId: string;
  until: string | null;
}

export function parseEmojiStatus(raw: unknown): EmojiStatus | null {
  if (!raw || typeof raw !== "object") return null;
  const value = raw as { stickerId?: unknown; until?: unknown };
  if (typeof value.stickerId !== "string" || !value.stickerId) return null;
  const until = typeof value.until === "string" && Number.isFinite(Date.parse(value.until)) ? value.until : null;
  return { stickerId: value.stickerId, until };
}

export function isEmojiStatusActive(status: EmojiStatus | null | undefined, now = Date.now()): status is EmojiStatus {
  if (!status) return false;
  return status.until === null || Date.parse(status.until) > now;
}

const MAX_TIMER_MS = 2 ** 31 - 1;

export function useActiveEmojiStatus(status: EmojiStatus | null | undefined): EmojiStatus | null {
  const [, tick] = React.useReducer((n: number) => n + 1, 0);
  const until = status?.until ?? null;
  React.useEffect(() => {
    if (!until) return;
    const left = Date.parse(until) - Date.now();
    if (left <= 0) return;
    const timer = setTimeout(tick, Math.min(left + 50, MAX_TIMER_MS));
    return () => clearTimeout(timer);
  }, [until]);
  return isEmojiStatusActive(status) ? status : null;
}
