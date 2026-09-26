import React from "react";

const LONG_PRESS_MS = 500;
const MOVE_TOLERANCE = 10;

export function useLongPress(
  onLongPress: (point: { clientX: number; clientY: number }, target: HTMLElement) => void,
) {
  const timer = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const start = React.useRef<{ x: number; y: number } | null>(null);
  const target = React.useRef<HTMLElement | null>(null);
  const fired = React.useRef(false);
  const callback = React.useRef(onLongPress);
  callback.current = onLongPress;

  const cancel = React.useCallback(() => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    start.current = null;
  }, []);

  React.useEffect(() => cancel, [cancel]);

  return {
    onTouchStart: (e: React.TouchEvent) => {
      if (e.touches.length !== 1) {
        cancel();
        return;
      }
      fired.current = false;
      target.current = e.currentTarget as HTMLElement;
      const touch = e.touches[0];
      start.current = { x: touch.clientX, y: touch.clientY };
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => {
        const point = start.current;
        const el = target.current;
        cancel();
        if (!point || !el) return;
        fired.current = true;
        callback.current({ clientX: point.x, clientY: point.y }, el);
      }, LONG_PRESS_MS);
    },
    onTouchMove: (e: React.TouchEvent) => {
      const origin = start.current;
      if (!origin) return;
      const touch = e.touches[0];
      if (Math.hypot(touch.clientX - origin.x, touch.clientY - origin.y) > MOVE_TOLERANCE) cancel();
    },
    onTouchEnd: (e?: React.TouchEvent) => {
      cancel();
      if (fired.current && e?.cancelable) e.preventDefault();
    },
    onTouchCancel: cancel,
    onClickCapture: (e: React.MouseEvent) => {
      if (!fired.current) return;
      fired.current = false;
      e.preventDefault();
      e.stopPropagation();
    },
  };
}
