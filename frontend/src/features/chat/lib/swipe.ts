export const SWIPE_TRIGGER_PX = 60;
export const SWIPE_MAX_PX = 80;
export const WHEEL_SWIPE_GAP_MS = 250;

export function touchSwipeOffset(dx: number, dy: number): number | null {
  if (Math.abs(dy) > Math.abs(dx)) return null;
  return Math.max(0, Math.min(SWIPE_MAX_PX, dx));
}

export interface WheelSwipe {
  accum: number;
  lastAt: number;
  fired: boolean;
}

export const IDLE_WHEEL_SWIPE: WheelSwipe = {
  accum: 0,
  lastAt: Number.NEGATIVE_INFINITY,
  fired: false,
};

export function nextWheelSwipe(
  prev: WheelSwipe,
  deltaX: number,
  deltaY: number,
  now: number,
): { state: WheelSwipe; fire: boolean } {
  const base = now - prev.lastAt > WHEEL_SWIPE_GAP_MS ? IDLE_WHEEL_SWIPE : prev;
  if (Math.abs(deltaY) >= Math.abs(deltaX)) {
    return { state: { ...base, lastAt: now }, fire: false };
  }
  const accum = Math.max(0, base.accum + deltaX);
  const fire = !base.fired && accum >= SWIPE_TRIGGER_PX;
  return { state: { accum, lastAt: now, fired: base.fired || fire }, fire };
}

export function wheelSwipeOffset(state: WheelSwipe): number {
  if (state.fired) return 0;
  return Math.min(state.accum, SWIPE_MAX_PX);
}
