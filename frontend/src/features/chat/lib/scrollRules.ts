export const WHEEL_GESTURE_GAP_MS = 200;
export const STICK_TO_BOTTOM_PX = 120;

export interface WheelGesture {
  lastAt: number;
  armed: boolean;
}

export const IDLE_WHEEL_GESTURE: WheelGesture = {
  lastAt: Number.NEGATIVE_INFINITY,
  armed: false,
};

export function nextWheelGesture(
  prev: WheelGesture,
  now: number,
  atBottom: boolean,
): WheelGesture {
  const fresh = now - prev.lastAt > WHEEL_GESTURE_GAP_MS;
  return { lastAt: now, armed: atBottom && (fresh || prev.armed) };
}

export function shouldStickToBottom(
  distanceFromBottom: number,
  ownMessage: boolean,
): boolean {
  return ownMessage || distanceFromBottom <= STICK_TO_BOTTOM_PX;
}

export function appendedOwnMessage(
  previousTailId: string | null,
  messages: { id: string; authorId: string }[],
  userId: string | undefined,
): boolean {
  const last = messages[messages.length - 1];
  if (!last || !userId || last.authorId !== userId) return false;
  if (last.id === previousTailId) return false;
  if (previousTailId === null) return true;
  return messages.slice(0, -1).some((m) => m.id === previousTailId);
}
