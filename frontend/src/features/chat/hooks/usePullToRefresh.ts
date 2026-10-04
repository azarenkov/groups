import React from "react";

import { IDLE_WHEEL_GESTURE, nextWheelGesture } from "../lib/scrollRules";

export const PULL_THRESHOLD = 64;
export const PULL_MAX = 96;
export const REFRESH_TIMEOUT_MS = 15000;
const MIN_SPIN_MS = 650;
const WHEEL_RELEASE_MS = 400;
const SETTLE = "0.3s cubic-bezier(0.22, 1, 0.36, 1)";

export const resist = (raw: number): number =>
  raw <= 0 ? 0 : PULL_MAX * (1 - Math.exp(-raw / (PULL_MAX * 1.2)));

const atBottom = (b: HTMLDivElement) => b.scrollTop + b.clientHeight >= b.scrollHeight - 1;

export function usePullToRefresh(params: {
  bodyRef: React.RefObject<HTMLDivElement>;
  enabled: boolean;
  convId?: string;
  onRefresh: () => Promise<void>;
}) {
  const { bodyRef, enabled, convId, onRefresh } = params;
  const indicatorRef = React.useRef<HTMLDivElement>(null);
  const distanceRef = React.useRef(0);
  const rawPullRef = React.useRef(0);
  const frameRef = React.useRef<number | null>(null);
  const refreshingRef = React.useRef(false);
  const inFlightRef = React.useRef(new Set<string | undefined>());
  const convIdRef = React.useRef(convId);
  convIdRef.current = convId;
  const touchStartYRef = React.useRef<number | null>(null);
  const wheelGestureRef = React.useRef(IDLE_WHEEL_GESTURE);
  const wheelReleaseRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const generationRef = React.useRef(0);
  const enabledRef = React.useRef(enabled);
  enabledRef.current = enabled;
  const onRefreshRef = React.useRef(onRefresh);
  onRefreshRef.current = onRefresh;

  const paint = React.useCallback(
    (distance: number, animate: boolean) => {
      distanceRef.current = distance;
      if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
      frameRef.current = requestAnimationFrame(() => {
        frameRef.current = null;
        const body = bodyRef.current;
        const indicator = indicatorRef.current;
        const shown = refreshingRef.current ? PULL_THRESHOLD : distance;
        const progress = Math.min(shown / PULL_THRESHOLD, 1);
        if (body) {
          body.style.transition = animate ? `transform ${SETTLE}` : "none";
          body.style.transform = shown > 0 ? `translate3d(0, ${-shown}px, 0)` : "";
          body.style.willChange = shown > 0 || refreshingRef.current ? "transform" : "";
        }
        if (indicator) {
          indicator.style.transition = animate ? `transform ${SETTLE}, opacity 0.2s` : "none";
          indicator.style.opacity = String(refreshingRef.current ? 1 : progress);
          indicator.style.transform = `translateX(-50%) translateY(${44 - shown}px) scale(${
            refreshingRef.current ? 1 : 0.6 + 0.4 * progress
          })`;
          indicator.classList.toggle("refreshing", refreshingRef.current);
          const icon = indicator.firstElementChild as HTMLElement | null;
          if (icon) icon.style.transform = refreshingRef.current ? "" : `rotate(${shown * 2.5}deg)`;
        }
      });
    },
    [bodyRef],
  );

  const clearWheelRelease = () => {
    if (wheelReleaseRef.current) clearTimeout(wheelReleaseRef.current);
    wheelReleaseRef.current = null;
  };

  const settle = React.useCallback(() => {
    touchStartYRef.current = null;
    paint(0, true);
  }, [paint]);

  React.useEffect(() => {
    generationRef.current += 1;
    refreshingRef.current = false;
    touchStartYRef.current = null;
    wheelGestureRef.current = IDLE_WHEEL_GESTURE;
    clearWheelRelease();
    paint(0, false);
  }, [convId, paint]);

  React.useEffect(
    () => () => {
      generationRef.current += 1;
      clearWheelRelease();
      if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
    },
    [],
  );

  const refresh = React.useCallback(async () => {
    const conv = convIdRef.current;
    if (refreshingRef.current || !enabledRef.current || inFlightRef.current.has(conv)) {
      settle();
      return;
    }
    const generation = generationRef.current;
    inFlightRef.current.add(conv);
    refreshingRef.current = true;
    touchStartYRef.current = null;
    clearWheelRelease();
    paint(PULL_THRESHOLD, true);
    const started = Date.now();
    let timer: ReturnType<typeof setTimeout> | null = null;
    try {
      const request = onRefreshRef.current().finally(() => {
        inFlightRef.current.delete(conv);
      });
      await Promise.race([
        request,
        new Promise<void>((resolve) => {
          timer = setTimeout(resolve, REFRESH_TIMEOUT_MS);
        }),
      ]);
    } catch {
    } finally {
      if (timer) clearTimeout(timer);
    }
    const wait = Math.max(0, MIN_SPIN_MS - (Date.now() - started));
    setTimeout(() => {
      if (generation !== generationRef.current) return;
      refreshingRef.current = false;
      wheelGestureRef.current = IDLE_WHEEL_GESTURE;
      paint(0, true);
    }, wait);
  }, [paint, settle]);

  const onPullTouchStart = (e: React.TouchEvent) => {
    if (refreshingRef.current) return;
    const b = bodyRef.current;
    touchStartYRef.current = b && atBottom(b) ? e.touches[0].clientY : null;
  };

  const onPullTouchMove = (e: React.TouchEvent) => {
    if (refreshingRef.current || touchStartYRef.current === null) return;
    const b = bodyRef.current;
    if (!b || !atBottom(b)) {
      settle();
      return;
    }
    const dy = touchStartYRef.current - e.touches[0].clientY;
    paint(resist(dy * 0.9), false);
  };

  const onPullTouchEnd = () => {
    if (refreshingRef.current || touchStartYRef.current === null) return;
    touchStartYRef.current = null;
    if (distanceRef.current >= PULL_THRESHOLD) void refresh();
    else settle();
  };

  const onPullWheel = (e: React.WheelEvent) => {
    if (refreshingRef.current) return;
    const b = bodyRef.current;
    if (!b) return;
    const gesture = nextWheelGesture(wheelGestureRef.current, e.timeStamp, atBottom(b));
    wheelGestureRef.current = gesture;
    if (!gesture.armed || e.deltaY <= 0) {
      if (!gesture.armed && distanceRef.current !== 0) {
        clearWheelRelease();
        settle();
      }
      return;
    }
    if (distanceRef.current === 0) rawPullRef.current = 0;
    rawPullRef.current += e.deltaY * 0.7;
    const next = resist(rawPullRef.current);
    if (next >= PULL_THRESHOLD) {
      distanceRef.current = next;
      void refresh();
      return;
    }
    paint(next, false);
    clearWheelRelease();
    wheelReleaseRef.current = setTimeout(() => {
      wheelReleaseRef.current = null;
      if (!refreshingRef.current) settle();
    }, WHEEL_RELEASE_MS);
  };

  return {
    indicatorRef,
    onPullTouchStart,
    onPullTouchMove,
    onPullTouchEnd,
    onPullTouchCancel: settle,
    onPullWheel,
  };
}
