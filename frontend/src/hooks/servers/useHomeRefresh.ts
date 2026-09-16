import { useEffect, useRef } from "react";

import { scheduleHomeRefresh, useHomeStore } from "../../store/homeStore";

interface HomeRefreshOptions {
  hasSession: boolean;
  homeActive: boolean;
  watched: unknown[];
}

export function useHomeRefresh({ hasSession, homeActive, watched }: HomeRefreshOptions) {
  const fetchHome = useHomeStore((s) => s.fetch);
  useEffect(() => {
    if (!hasSession || !homeActive) return;
    void fetchHome();
    const interval = window.setInterval(() => void fetchHome(), 60_000);
    const onFocus = () => void fetchHome();
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener("focus", onFocus);
    };
  }, [hasSession, homeActive, fetchHome]);
  const homeRefreshArmedRef = useRef(false);
  useEffect(() => {
    if (!homeActive) {
      homeRefreshArmedRef.current = false;
      return;
    }
    if (!homeRefreshArmedRef.current) {
      homeRefreshArmedRef.current = true;
      return;
    }
    scheduleHomeRefresh();
  }, [homeActive, watched]);


  return { fetchHome };
}
