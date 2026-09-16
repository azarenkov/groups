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

function tone(
  ac: AudioContext,
  dest: AudioNode,
  freq: number,
  start: number,
  duration: number,
  peak: number,
) {
  const osc = ac.createOscillator();
  osc.type = "sine";
  osc.frequency.setValueAtTime(freq, start);
  osc.frequency.exponentialRampToValueAtTime(freq * 1.004, start + duration);
  const gain = ac.createGain();
  gain.gain.setValueAtTime(0.0001, start);
  gain.gain.exponentialRampToValueAtTime(peak, start + 0.012);
  gain.gain.exponentialRampToValueAtTime(0.0001, start + duration);
  osc.connect(gain);
  gain.connect(dest);
  osc.start(start);
  osc.stop(start + duration + 0.02);
}

export function playNotificationSound(): void {
  const now = Date.now();
  if (now - lastPlayedAt < MIN_INTERVAL_MS) return;
  const ac = getContext();
  if (!ac) return;
  lastPlayedAt = now;

  const master = ac.createGain();
  master.gain.value = 0.11;
  master.connect(ac.destination);

  const t = ac.currentTime + 0.02;
  tone(ac, master, 830.6, t, 0.28, 0.9);
  tone(ac, master, 1108.7, t + 0.07, 0.34, 0.75);
  tone(ac, master, 1661.2, t + 0.07, 0.22, 0.12);
}
