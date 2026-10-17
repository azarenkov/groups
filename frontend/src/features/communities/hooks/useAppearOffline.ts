import { useCallback, useEffect, useRef, useState } from "react";
import { userAPI } from "../../social/api/user";
import { useAuthStore } from "../../auth/store/authStore";
import { usePresenceStore } from "../../social/store/presenceStore";
import { setActivityHiddenByPresence } from "../../social/lib/gameActivity";

const RETRY_MS = 5000;
const CACHE_KEY = "rift:appearOffline";

function readCached(): boolean {
  try {
    return localStorage.getItem(CACHE_KEY) === "1";
  } catch {
    return false;
  }
}

function writeCached(value: boolean) {
  try {
    localStorage.setItem(CACHE_KEY, value ? "1" : "0");
  } catch {}
}

export function useAppearOffline(params: {
  hasSession: boolean;
  currentUserId?: string;
  serverAppearOffline?: boolean;
}) {
  const { hasSession, currentUserId, serverAppearOffline } = params;
  const [attempt, setAttempt] = useState(0);
  const [pending, setPending] = useState<boolean | null>(null);
  const retry = useRef<number | null>(null);
  const inFlight = useRef(false);
  const [appearOffline, setAppearOffline] = useState<boolean>(readCached);

  useEffect(() => {
    if (serverAppearOffline === undefined || pending !== null) return;
    setAppearOffline(serverAppearOffline);
    writeCached(serverAppearOffline);
  }, [serverAppearOffline, pending]);

  useEffect(() => {
    if (!hasSession || pending === null || inFlight.current) return;
    if (retry.current !== null) {
      window.clearTimeout(retry.current);
      retry.current = null;
    }
    const target = pending;
    let failed = false;
    inFlight.current = true;
    userAPI
      .setAppearOffline(target)
      .then(() => {
        const auth = useAuthStore.getState();
        if (auth.user) auth.setUser({ ...auth.user, appearOffline: target });
        setPending((current) => (current === target ? null : current));
      })
      .catch((err) => {
        console.warn("[presence] setAppearOffline failed:", err);
        failed = true;
      })
      .finally(() => {
        inFlight.current = false;
        if (failed) {
          retry.current = window.setTimeout(() => setAttempt((n) => n + 1), RETRY_MS);
        } else {
          setAttempt((n) => n + 1);
        }
      });
  }, [pending, hasSession, attempt]);

  useEffect(() => {
    setActivityHiddenByPresence(appearOffline);
  }, [appearOffline]);

  useEffect(() => {
    if (!currentUserId) return;
    usePresenceStore.getState().setSelf(currentUserId, !appearOffline);
  }, [currentUserId, appearOffline]);

  const setStatus = useCallback((status: "online" | "offline") => {
    const next = status === "offline";
    setAppearOffline(next);
    writeCached(next);
    setPending(next);
  }, []);

  return { userStatus: appearOffline ? "offline" : "online", setStatus };
}
