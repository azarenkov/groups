import { useCallback, useEffect, useState } from "react";
import { userAPI } from "../../social/api/user";
import { usePresenceStore } from "../../social/store/presenceStore";

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
    userAPI.setAppearOffline(appearOffline).catch((err) => {
      console.warn("[presence] setAppearOffline failed:", err);
    });
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
