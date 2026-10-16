import { mediaAPI, type MediaState } from "../api/media";
import { nextMediaRevision, type MediaRevision } from "./mediaRevision";

const SAVE_DELAY_MS = 300;
const RETRY_DELAY_MS = 5000;

const keyOf = (state: MediaState) => `${state.micMuted}:${state.deafened}`;

let desired: { state: MediaState; revision: MediaRevision } | null = null;
let saved: string | null = null;
let inFlight: string | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;
let generation = 0;

const schedule = (delay: number) => {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => flush(), delay);
};

const flush = (keepalive = false) => {
  if (timer) clearTimeout(timer);
  timer = null;
  const next = desired;
  if (!next || inFlight || keyOf(next.state) === saved) return;
  const key = keyOf(next.state);
  const sentIn = generation;
  inFlight = key;
  mediaAPI.save(next.state, next.revision, { keepalive }).then(
    () => {
      if (sentIn !== generation) return;
      inFlight = null;
      saved = key;
      if (desired && keyOf(desired.state) !== saved) flush();
    },
    (err) => {
      if (sentIn !== generation) return;
      inFlight = null;
      console.warn("[voice] failed to save mic and sound state:", err);
      schedule(RETRY_DELAY_MS);
    },
  );
};

export function syncMediaState(state: MediaState): void {
  desired = { state, revision: nextMediaRevision() };
  schedule(SAVE_DELAY_MS);
}

let unloadFlushInstalled = false;

export function installMediaStateFlushOnUnload(): void {
  if (unloadFlushInstalled || typeof window === "undefined") return;
  unloadFlushInstalled = true;
  window.addEventListener("pagehide", () => {
    inFlight = null;
    flush(true);
  });
}

export function loadMediaState(): Promise<MediaState | null> {
  return mediaAPI.get().catch((err) => {
    console.warn("[voice] failed to load mic and sound state:", err);
    return null;
  });
}

export function resetMediaStateSync(): void {
  generation += 1;
  if (timer) clearTimeout(timer);
  timer = null;
  desired = null;
  saved = null;
  inFlight = null;
}
