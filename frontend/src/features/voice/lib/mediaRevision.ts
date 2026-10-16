import { randomUuid } from "../../../shared/lib/ids";

const SOURCE_KEY = "rift.voiceMediaSource.v1";

export interface MediaRevision {
  source: string;
  seq: number;
}

let source: string | null = null;
let lastSeq = 0;

const tabSource = (): string => {
  if (source) return source;
  try {
    source = sessionStorage.getItem(SOURCE_KEY);
  } catch {}
  if (!source) {
    source = randomUuid();
    try {
      sessionStorage.setItem(SOURCE_KEY, source);
    } catch {}
  }
  return source;
};

export function nextMediaRevision(now = Date.now()): MediaRevision {
  lastSeq = Math.max(now, lastSeq + 1);
  return { source: tabSource(), seq: lastSeq };
}
