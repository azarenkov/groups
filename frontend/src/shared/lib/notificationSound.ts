import { MESSAGE_CHIME, playChime, type ChimeNote } from "./chime";

const NOTIFICATION_VOLUME = 0.12;
const NOTIFICATION_INTERVAL_MS = 1500;
const VOICE_CUE_INTERVAL_MS = 120;

let ctx: AudioContext | null = null;
const lastPlayedAt: Record<string, number> = {};

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

function play(lane: string, chime: ChimeNote[], minIntervalMs: number): void {
  const now = Date.now();
  if (now - (lastPlayedAt[lane] ?? 0) < minIntervalMs) return;
  const ac = getContext();
  if (!ac) return;
  lastPlayedAt[lane] = now;
  playChime(ac, chime, NOTIFICATION_VOLUME);
}

export function playNotificationSound(chime: ChimeNote[] = MESSAGE_CHIME): void {
  play("notification", chime, NOTIFICATION_INTERVAL_MS);
}

export function playVoiceCue(chime: ChimeNote[]): void {
  play("voice", chime, VOICE_CUE_INTERVAL_MS);
}
