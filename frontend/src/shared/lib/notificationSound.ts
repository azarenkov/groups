import { MESSAGE_CHIME, playChime } from "./chime";

let ctx: AudioContext | null = null;
let lastPlayedAt = 0;

const MIN_INTERVAL_MS = 1500;

function getContext(): AudioContext | null {
  try {
    if (!ctx) {
      const Ctor =
        window.AudioContext ??
        (window as unknown as { webkitAudioContext?: typeof AudioContext })
          .webkitAudioContext;
      if (!Ctor) return null;
      ctx = new Ctor();
    }
    if (ctx.state === "suspended") void ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

export function playNotificationSound(): void {
  const now = Date.now();
  if (now - lastPlayedAt < MIN_INTERVAL_MS) return;
  const ac = getContext();
  if (!ac) return;
  lastPlayedAt = now;
  playChime(ac, MESSAGE_CHIME, 0.12);
}
