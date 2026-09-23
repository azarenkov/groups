import { useCallback, useEffect, useState } from "react";
import { userAPI } from "../../social/api/user";
import { usePresenceStore } from "../../social/store/presenceStore";
import { setActivityHiddenByPresence } from "../../social/lib/gameActivity";

export function useAppearOffline(params: {
  hasSession: boolean;
  currentUserId?: string;
}) {
  const { hasSession, currentUserId } = params;
  const [appearOffline, setAppearOffline] = useState<boolean>(() => {
    try {
      return localStorage.getItem("rift:appearOffline") === "1";
    } catch {
      return false;
    }
  });

  useEffect(() => {
    if (!hasSession) return;
    let cancelled = false;
    if (appearOffline) setActivityHiddenByPresence(true);
    userAPI
      .setAppearOffline(appearOffline)
      .then(() => {
        if (!cancelled && !appearOffline) setActivityHiddenByPresence(false);
      })
      .catch((err) => {
        console.warn("[presence] setAppearOffline failed:", err);
        if (!cancelled && !appearOffline) setActivityHiddenByPresence(false);
      });
    return () => {
      cancelled = true;
    };
  }, [appearOffline, hasSession]);

  useEffect(() => {
    if (!currentUserId) return;
    usePresenceStore.getState().setSelf(currentUserId, !appearOffline);
  }, [currentUserId, appearOffline]);


  const setStatus = useCallback((status: "online" | "offline") => {
    const next = status === "offline";
    setAppearOffline(next);
    try {
      localStorage.setItem("rift:appearOffline", next ? "1" : "0");
    } catch {}
  }, []);

  return { userStatus: appearOffline ? "offline" : "online", setStatus };
}
