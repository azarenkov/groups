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
  replacement: string | null;
}

export function followWatched(
  shares: ScreenShareInfo[],
  watched: string[],
  held: HeldScreen | null,
  now: number,
): { watched: string[]; held: HeldScreen | null } {
  const present = (identity: string) =>
    shares.some((share) => share.identity === identity);
  const kept = watched.filter(present);
  const primary = watched[0] ?? null;
  if (primary !== null && !present(primary)) {
    const replacement = kept.length > 0 ? null : nextWatched(shares, primary);
    return {
      watched: replacement === null ? kept : [replacement],
      held: { identity: primary, until: now + SCREEN_HOLD_MS, replacement },
    };
  }
  const hold = held && held.until > now ? held : null;
  if (hold && present(hold.identity) && !kept.includes(hold.identity)) {
    return {
      watched: [hold.identity, ...kept.filter((id) => id !== hold.replacement)],
      held: null,
    };
  }
  return { watched: kept, held: hold && !kept.includes(hold.identity) ? hold : null };
}
