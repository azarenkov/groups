export interface ChimeNote {
  freq: number;
  at: number;
  peak?: number;
  decay?: number;
}

const PARTIALS = [
  { ratio: 1, gain: 1, decay: 1 },
  { ratio: 2, gain: 0.32, decay: 0.55 },
  { ratio: 3.01, gain: 0.12, decay: 0.35 },
  { ratio: 4.2, gain: 0.05, decay: 0.22 },
];

const ECHO_TAIL_S = 1.2;

function echoBus(ac: BaseAudioContext, dest: AudioNode): { input: AudioNode; nodes: AudioNode[] } {
  const input = ac.createGain();
  const tone = ac.createBiquadFilter();
  tone.type = "lowpass";
  tone.frequency.value = 5200;
  tone.Q.value = 0.4;
  input.connect(tone);
  tone.connect(dest);

  const delay = ac.createDelay(1);
  delay.delayTime.value = 0.13;
  const feedback = ac.createGain();
  feedback.gain.value = 0.22;
  const wet = ac.createGain();
  wet.gain.value = 0.28;
  const damp = ac.createBiquadFilter();
  damp.type = "lowpass";
  damp.frequency.value = 2600;
  tone.connect(delay);
  delay.connect(damp);
  damp.connect(feedback);
  feedback.connect(delay);
  damp.connect(wet);
  wet.connect(dest);
  return { input, nodes: [input, tone, delay, feedback, wet, damp] };
}

function mallet(ac: BaseAudioContext, dest: AudioNode, note: ChimeNote, start: number) {
  const peak = note.peak ?? 1;
  const decay = note.decay ?? 0.9;
  for (const p of PARTIALS) {
    const osc = ac.createOscillator();
    osc.type = "sine";
    osc.frequency.setValueAtTime(note.freq * p.ratio, start);
    const gain = ac.createGain();
    const length = decay * p.decay;
    gain.gain.setValueAtTime(0.0001, start);
    gain.gain.exponentialRampToValueAtTime(peak * p.gain, start + 0.006);
    gain.gain.exponentialRampToValueAtTime(0.0001, start + length);
    osc.connect(gain);
    gain.connect(dest);
    osc.start(start);
    osc.stop(start + length + 0.05);
  }
}

export function playChime(ac: BaseAudioContext, notes: ChimeNote[], volume: number, offset = 0.02): number {
  const master = ac.createGain();
  master.gain.value = volume;
  master.connect(ac.destination);
  const bus = echoBus(ac, master);
  const start = ac.currentTime + offset;
  let end = 0;
  for (const note of notes) {
    mallet(ac, bus.input, note, start + note.at);
    end = Math.max(end, note.at + (note.decay ?? 0.9));
  }
  const offline = typeof OfflineAudioContext !== "undefined" && ac instanceof OfflineAudioContext;
  if (typeof window !== "undefined" && !offline) {
    window.setTimeout(() => {
      for (const node of bus.nodes) node.disconnect();
      master.disconnect();
    }, (offset + end + ECHO_TAIL_S) * 1000);
  }
  return end;
}

export const MESSAGE_CHIME: ChimeNote[] = [
  { freq: 880, at: 0, peak: 0.8, decay: 0.55 },
  { freq: 1318.51, at: 0.085, peak: 0.7, decay: 0.85 },
];

export const RING_CHIME: ChimeNote[] = [
  { freq: 659.25, at: 0, peak: 0.75, decay: 0.7 },
  { freq: 830.61, at: 0.13, peak: 0.7, decay: 0.7 },
  { freq: 987.77, at: 0.26, peak: 0.7, decay: 0.7 },
  { freq: 1318.51, at: 0.39, peak: 0.8, decay: 1.1 },
  { freq: 987.77, at: 0.82, peak: 0.55, decay: 0.6 },
  { freq: 1318.51, at: 0.95, peak: 0.65, decay: 1.2 },
];

export const RING_INTERVAL_MS = 2800;

export const DECLINED_CHIME: ChimeNote[] = [
  { freq: 659.25, at: 0, peak: 0.7, decay: 0.45 },
  { freq: 523.25, at: 0.16, peak: 0.65, decay: 0.45 },
  { freq: 392, at: 0.32, peak: 0.6, decay: 0.8 },
];
