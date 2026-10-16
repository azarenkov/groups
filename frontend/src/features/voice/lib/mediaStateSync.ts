import { mediaAPI, type MediaState } from "../api/media";

const SAVE_DELAY_MS = 300;

let lastSaved: string | null = null;
let pending: MediaState | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;

const flush = (keepalive = false) => {
  if (timer) clearTimeout(timer);
  timer = null;
  const state = pending;
  pending = null;
  if (!state) return;
  const key = `${state.micMuted}:${state.deafened}`;
  if (key === lastSaved) return;
  lastSaved = key;
  mediaAPI.save(state, { keepalive }).catch((err) => {
    if (lastSaved === key) lastSaved = null;
    console.warn("[voice] failed to save mic and sound state:", err);
  });
};

export function syncMediaState(state: MediaState): void {
  pending = state;
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => flush(), SAVE_DELAY_MS);
}

let unloadFlushInstalled = false;

export function installMediaStateFlushOnUnload(): void {
  if (unloadFlushInstalled || typeof window === "undefined") return;
  unloadFlushInstalled = true;
  window.addEventListener("pagehide", () => flush(true));
}

export function loadMediaState(): Promise<MediaState | null> {
  return mediaAPI.get().catch((err) => {
    console.warn("[voice] failed to load mic and sound state:", err);
    return null;
  });
}

export function resetMediaStateSync(): void {
  if (timer) clearTimeout(timer);
  timer = null;
  pending = null;
  lastSaved = null;
}
