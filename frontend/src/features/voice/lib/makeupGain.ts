export const MAKEUP_INTERVAL_MS = 250;
export const MAKEUP_BASE = 2;
export const MAKEUP_MAX = 4;
export const MAKEUP_SPEECH_FLOOR = 0.01;
export const MAKEUP_SPEECH_BAND = 0.45;
export const MAKEUP_SMOOTHING = 0.2;
export const MAKEUP_RELEASE = 0.35;
export const SPEECH_BAND_LOW_HZ = 300;
export const SPEECH_BAND_HIGH_HZ = 3400;

export function speechBandRatio(
  spectrum: Uint8Array,
  sampleRate: number,
): number {
  const binHz = sampleRate / 2 / spectrum.length;
  let total = 0;
  let band = 0;
  for (let i = 0; i < spectrum.length; i++) {
    const energy = spectrum[i] * spectrum[i];
    total += energy;
    const hz = (i + 0.5) * binHz;
    if (hz >= SPEECH_BAND_LOW_HZ && hz <= SPEECH_BAND_HIGH_HZ) band += energy;
  }
  return total > 0 ? band / total : 0;
}

export function isSpeech(
  inputRms: number,
  bandRatio: number,
  gateOpen: boolean,
): boolean {
  if (!gateOpen || inputRms < MAKEUP_SPEECH_FLOOR) return false;
  return bandRatio >= MAKEUP_SPEECH_BAND;
}

export function makeupGainFor(
  inputRms: number,
  denoisedRms: number,
  current: number,
  speech: boolean,
  gateOpen: boolean = true,
): number {
  if (!gateOpen) return MAKEUP_BASE;
  if (!speech || denoisedRms <= 0) {
    return current + (MAKEUP_BASE - current) * MAKEUP_RELEASE;
  }
  const wanted = Math.min(
    MAKEUP_MAX,
    Math.max(MAKEUP_BASE, inputRms / denoisedRms),
  );
  return current + (wanted - current) * MAKEUP_SMOOTHING;
}
