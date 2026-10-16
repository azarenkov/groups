import { randomUuid } from "../../../shared/lib/ids";

const KEY = "rift.voiceMediaRevision.v1";

export interface MediaRevision {
  source: string;
  seq: number;
}

let last: MediaRevision | null = null;

const stored = (): MediaRevision | null => {
  try {
    const value = JSON.parse(sessionStorage.getItem(KEY) ?? "null") as MediaRevision | null;
    return value && typeof value.source === "string" && Number.isFinite(value.seq) ? value : null;
  } catch {
    return null;
  }
};

export function nextMediaRevision(now = Date.now()): MediaRevision {
  const previous = last ?? stored();
  last = { source: previous?.source ?? randomUuid(), seq: Math.max(now, (previous?.seq ?? 0) + 1) };
  try {
    sessionStorage.setItem(KEY, JSON.stringify(last));
  } catch {}
  return last;
}
