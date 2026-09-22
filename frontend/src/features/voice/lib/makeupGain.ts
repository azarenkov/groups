export const MAKEUP_INTERVAL_MS = 250;
export const MAKEUP_MIN = 1;
export const MAKEUP_MAX = 6;
export const MAKEUP_SPEECH_FLOOR = 0.01;
export const MAKEUP_SMOOTHING = 0.2;

export function makeupGainFor(
  inputRms: number,
  denoisedRms: number,
  current: number,
): number {
  if (inputRms < MAKEUP_SPEECH_FLOOR || denoisedRms <= 0) return current;
  const wanted = Math.min(
    MAKEUP_MAX,
    Math.max(MAKEUP_MIN, inputRms / denoisedRms),
  );
  return current + (wanted - current) * MAKEUP_SMOOTHING;
}
