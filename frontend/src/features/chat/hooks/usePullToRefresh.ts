import React from "react";

import { IDLE_WHEEL_GESTURE, nextWheelGesture } from "../lib/scrollRules";

export const PULL_THRESHOLD = 64;
export const PULL_MAX = 96;
export const REFRESH_TIMEOUT_MS = 15000;
const MIN_SPIN_MS = 650;
const WHEEL_RELEASE_MS = 400;
export const SPRING_STIFFNESS = 170;
export const SPRING_DAMPING = 2 * Math.sqrt(SPRING_STIFFNESS);
const MAX_STEP_S = 1 / 30;

type Motion = "none" | "follow" | "settle";

export function springStep(
  position: number,
  velocity: number,
  target: number,
  dt: number,
): { position: number; velocity: number; done: boolean } {
  const accel = SPRING_STIFFNESS * (target - position) - SPRING_DAMPING * velocity;
  const nextVelocity = velocity + accel * dt;
  const nextPosition = position + nextVelocity * dt;
  if (Math.abs(target - nextPosition) < 0.25 && Math.abs(nextVelocity) < 4) {
    return { position: target, velocity: 0, done: true };
  }
  return { position: nextPosition, velocity: nextVelocity, done: false };
}

export const resist = (raw: number): number =>
  raw <= 0 ? 0 : PULL_MAX * (1 - Math.exp(-raw / (PULL_MAX * 1.2)));

export const prefersReducedMotion = (): boolean =>
  document.documentElement.classList.contains("reduce-motion") ||
  (typeof window.matchMedia === "function" && window.matchMedia("(prefers-reduced-motion: reduce)").matches);

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
  const shownRef = React.useRef(0);
  const velocityRef = React.useRef(0);
  const lastFrameRef = React.useRef<number | null>(null);
  const directPendingRef = React.useRef<number | null>(null);
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

  const render = React.useCallback(
    (shown: number) => {
      const body = bodyRef.current;
      const indicator = indicatorRef.current;
      const refreshing = refreshingRef.current;
      const progress = Math.min(Math.max(shown, 0) / PULL_THRESHOLD, 1);
      if (body) {
        body.style.transition = "none";
        body.style.transform = shown > 0.5 ? `translate3d(0, ${-shown}px, 0)` : "";
        body.style.willChange = shown > 0.5 || refreshing ? "transform" : "";
      }
      if (indicator) {
        indicator.style.transition = "opacity 0.2s";
        indicator.style.opacity = String(refreshing ? 1 : progress);
        indicator.style.transform = `translateX(-50%) translateY(${44 - shown}px) scale(${
          refreshing ? 1 : 0.6 + 0.4 * progress
        })`;
        indicator.classList.toggle("refreshing", refreshing);
        const icon = indicator.firstElementChild as HTMLElement | null;
        if (icon) icon.style.transform = refreshing ? "" : `rotate(${shown * 2.5}deg)`;
      }
    },
    [bodyRef],
  );

  const paint = React.useCallback(
    (distance: number, motion: Motion) => {
      distanceRef.current = distance;
      const targetNow = () => (refreshingRef.current ? PULL_THRESHOLD : distanceRef.current);
      if (motion === "none" || prefersReducedMotion()) {
        if (frameRef.current !== null) cancelAnimationFrame(frameRef.current);
        lastFrameRef.current = null;
        directPendingRef.current = targetNow();
        frameRef.current = requestAnimationFrame(() => {
          frameRef.current = null;
          directPendingRef.current = null;
          shownRef.current = targetNow();
          velocityRef.current = 0;
          render(shownRef.current);
        });
        return;
      }
      if (directPendingRef.current !== null && frameRef.current !== null) {
        cancelAnimationFrame(frameRef.current);
        frameRef.current = null;
        shownRef.current = directPendingRef.current;
        velocityRef.current = 0;
        directPendingRef.current = null;
      }
      if (frameRef.current !== null) return;
      const step = (now: number) => {
        const last = lastFrameRef.current ?? now - 1000 / 60;
        const dt = Math.min(Math.max((now - last) / 1000, 0.001), MAX_STEP_S);
        lastFrameRef.current = now;
        const next = springStep(shownRef.current, velocityRef.current, targetNow(), dt);
        shownRef.current = next.position;
        velocityRef.current = next.velocity;
        render(next.position);
        if (next.done) {
          frameRef.current = null;
          lastFrameRef.current = null;
          return;
        }
        frameRef.current = requestAnimationFrame(step);
      };
      frameRef.current = requestAnimationFrame(step);
    },
    [render],
  );

  const clearWheelRelease = () => {
    if (wheelReleaseRef.current) clearTimeout(wheelReleaseRef.current);
    wheelReleaseRef.current = null;
  };

  const settle = React.useCallback(() => {
    touchStartYRef.current = null;
    paint(0, "settle");
  }, [paint]);

  React.useEffect(() => {
    generationRef.current += 1;
    refreshingRef.current = false;
    touchStartYRef.current = null;
    wheelGestureRef.current = IDLE_WHEEL_GESTURE;
    clearWheelRelease();
    paint(0, "none");
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
    paint(PULL_THRESHOLD, "settle");
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
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          if (generation !== generationRef.current) return;
          refreshingRef.current = false;
          wheelGestureRef.current = { lastAt: wheelGestureRef.current.lastAt, armed: false };
          paint(0, "settle");
        }),
      );
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
    paint(resist(dy * 0.9), "none");
  };

  const onPullTouchEnd = () => {
    if (refreshingRef.current || touchStartYRef.current === null) return;
    touchStartYRef.current = null;
    if (distanceRef.current >= PULL_THRESHOLD) void refresh();
    else settle();
  };

  const onPullWheel = (e: React.WheelEvent) => {
    if (refreshingRef.current) {
      wheelGestureRef.current = { lastAt: e.timeStamp, armed: false };
      return;
    }
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
    paint(next, "follow");
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
