import React from "react";

export const PULL_THRESHOLD = 64;
export const PULL_MAX = 96;

export function usePullToRefresh(params: {
  bodyRef: React.RefObject<HTMLDivElement>;
  enabled: boolean;
  convId?: string;
  onRefresh: () => Promise<void>;
}) {
  const { bodyRef, enabled, convId, onRefresh } = params;
  const [pullDistance, setPullDistance] = React.useState(0);
  const [refreshing, setRefreshing] = React.useState(false);
  const pullStartYRef = React.useRef<number | null>(null);
  const wheelPullRef = React.useRef(0);
  const wheelResetTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );
  const refreshingRef = React.useRef(false);
  refreshingRef.current = refreshing;
  const enabledRef = React.useRef(enabled);
  enabledRef.current = enabled;
  const onRefreshRef = React.useRef(onRefresh);
  onRefreshRef.current = onRefresh;

  React.useEffect(() => {
    setPullDistance(0);
    pullStartYRef.current = null;
    wheelPullRef.current = 0;
  }, [convId]);

  const triggerPullRefresh = React.useCallback(async () => {
    if (refreshingRef.current || !enabledRef.current) {
      setPullDistance(0);
      return;
    }
    setRefreshing(true);
    setPullDistance(PULL_THRESHOLD);
    const started = Date.now();
    try {
      await onRefreshRef.current();
    } finally {
      const wait = Math.max(0, 650 - (Date.now() - started));
      setTimeout(() => {
        setRefreshing(false);
        setPullDistance(0);
        wheelPullRef.current = 0;
      }, wait);
    }
  }, []);

  const atBottom = (b: HTMLDivElement) =>
    b.scrollTop + b.clientHeight >= b.scrollHeight - 1;

  const onPullTouchStart = (e: React.TouchEvent) => {
    if (refreshingRef.current) return;
    const b = bodyRef.current;
    pullStartYRef.current = b && atBottom(b) ? e.touches[0].clientY : null;
  };

  const onPullTouchMove = (e: React.TouchEvent) => {
    if (refreshingRef.current || pullStartYRef.current === null) return;
    const b = bodyRef.current;
    if (!b || !atBottom(b)) {
      pullStartYRef.current = null;
      setPullDistance(0);
      return;
    }
    const dy = pullStartYRef.current - e.touches[0].clientY;
    setPullDistance(dy > 0 ? Math.min(dy * 0.45, PULL_MAX) : 0);
  };

  const onPullTouchEnd = () => {
    if (refreshingRef.current) return;
    pullStartYRef.current = null;
    if (pullDistance >= PULL_THRESHOLD) {
      void triggerPullRefresh();
    } else {
      setPullDistance(0);
    }
  };

  const onPullWheel = (e: React.WheelEvent) => {
    if (refreshingRef.current) return;
    const b = bodyRef.current;
    if (!b || !atBottom(b) || e.deltaY <= 0) {
      if (wheelPullRef.current !== 0 && b && !atBottom(b)) {
        wheelPullRef.current = 0;
        setPullDistance(0);
      }
      return;
    }
    wheelPullRef.current = Math.min(
      wheelPullRef.current + e.deltaY * 0.35,
      PULL_MAX,
    );
    setPullDistance(wheelPullRef.current);
    if (wheelResetTimerRef.current) clearTimeout(wheelResetTimerRef.current);
    if (wheelPullRef.current >= PULL_THRESHOLD) {
      void triggerPullRefresh();
      return;
    }
    wheelResetTimerRef.current = setTimeout(() => {
      wheelResetTimerRef.current = null;
      if (!refreshingRef.current) {
        wheelPullRef.current = 0;
        setPullDistance(0);
      }
    }, 400);
  };

  return {
    pullDistance,
    refreshing,
    pullStartYRef,
    onPullTouchStart,
    onPullTouchMove,
    onPullTouchEnd,
    onPullWheel,
  };
}
