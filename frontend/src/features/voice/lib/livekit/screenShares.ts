import type { ScreenShareInfo } from "./types";

export function nextWatched(
  shares: ScreenShareInfo[],
  current: string | null,
): string | null {
  if (current === null) return null;
  if (shares.some((share) => share.identity === current)) return current;
  const local = shares.find((share) => share.isLocal);
  const remote = shares.find((share) => !share.isLocal);
  return remote?.identity ?? local?.identity ?? null;
}

export const SCREEN_HOLD_MS = 5000;

export interface HeldScreen {
  identity: string;
  until: number;
}

export function followWatched(
  shares: ScreenShareInfo[],
  current: string | null,
  held: HeldScreen | null,
  now: number,
): { watched: string | null; held: HeldScreen | null } {
  const present = (identity: string) =>
    shares.some((share) => share.identity === identity);
  let hold = held && held.until > now ? held : null;
  if (current !== null && !present(current)) {
    hold = { identity: current, until: now + SCREEN_HOLD_MS };
  }
  if (hold && present(hold.identity)) {
    return { watched: hold.identity, held: null };
  }
  return { watched: nextWatched(shares, current), held: hold };
}
