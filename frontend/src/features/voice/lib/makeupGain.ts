export const MAKEUP_INTERVAL_MS = 250;
export const MAKEUP_MIN = 1;
export const MAKEUP_MAX = 6;
export const MAKEUP_SPEECH_FLOOR = 0.01;
export const MAKEUP_SMOOTHING = 0.2;
export const MAKEUP_RELEASE = 0.05;
export const MAKEUP_NOISE_MARGIN = 3;
export const NOISE_FLOOR_FALL = 0.3;
export const NOISE_FLOOR_RISE = 0.02;

export function updateNoiseFloor(floor: number, inputRms: number): number {
  const rate = inputRms < floor ? NOISE_FLOOR_FALL : NOISE_FLOOR_RISE;
  return floor + (inputRms - floor) * rate;
}

export function isSpeech(
  inputRms: number,
  noiseFloor: number,
  gateOpen: boolean,
): boolean {
  return (
    gateOpen &&
    inputRms >= MAKEUP_SPEECH_FLOOR &&
    inputRms > noiseFloor * MAKEUP_NOISE_MARGIN
  );
}

export function makeupGainFor(
  inputRms: number,
  denoisedRms: number,
  current: number,
  speech: boolean,
): number {
  if (!speech || denoisedRms <= 0) {
    return current + (MAKEUP_MIN - current) * MAKEUP_RELEASE;
  }
  const wanted = Math.min(
    MAKEUP_MAX,
    Math.max(MAKEUP_MIN, inputRms / denoisedRms),
  );
  return current + (wanted - current) * MAKEUP_SMOOTHING;
}
