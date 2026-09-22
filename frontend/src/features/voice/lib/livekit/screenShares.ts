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
