export const GIF_SIZE = 256;
export const GIF_MAX_BYTES = 8 * 1024 * 1024;
export const GIF_MAX_LENGTH_MS = 5000;
export const GIF_MIN_LENGTH_MS = 1000;
export const GIF_FRAME_MS = 70;
export const GIF_FRAME_STEPS = [1, 2, 3];

export const clampWindow = (durationMs: number, startMs: number, lengthMs: number) => {
  const length = Math.max(0, Math.min(GIF_MAX_LENGTH_MS, lengthMs, durationMs));
  const start = Math.max(0, Math.min(startMs, durationMs - length));
  return { startMs: start, lengthMs: length };
};

export const frameTimes = (startMs: number, lengthMs: number, delayMs: number): number[] => {
  const count = Math.max(1, Math.floor(lengthMs / delayMs));
  return Array.from({ length: count }, (_, i) => startMs + i * delayMs);
};
