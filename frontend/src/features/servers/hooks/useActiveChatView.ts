import { useEffect, useState } from "react";
import { setActiveChatView } from "../../../shared/ws/websocketBootstrap";
import { useUnreadStore } from "../../chat/store/unreadStore";
import { userAPI } from "../../social/api/user";

export function useActiveChatView(params: {
  textChannelId: string | null;
  dmId: string | null;
  blocked: boolean;
}) {
  const { textChannelId, dmId, blocked } = params;
  const [windowFocused, setWindowFocused] = useState(
    () => typeof document === "undefined" || document.hasFocus(),
  );
  useEffect(() => {
    const update = () =>
      setWindowFocused(document.hasFocus() && !document.hidden);
    window.addEventListener("focus", update);
    window.addEventListener("blur", update);
    document.addEventListener("visibilitychange", update);
    return () => {
      window.removeEventListener("focus", update);
      window.removeEventListener("blur", update);
      document.removeEventListener("visibilitychange", update);
    };
  }, []);

  const hidden = !windowFocused || blocked;
  const visibleTextChannelId = hidden ? null : textChannelId;
  const visibleDmId = hidden ? null : dmId;

  useEffect(() => {
    if (visibleDmId) {
      setActiveChatView({ kind: "dm", id: visibleDmId });
      if ((useUnreadStore.getState().byDm[visibleDmId] ?? 0) > 0) {
        useUnreadStore.getState().resetDm(visibleDmId);
        userAPI.markDmRead(visibleDmId).catch((err) => {
          console.warn("[read-state] mark dm read failed:", err);
        });
      }
    } else if (visibleTextChannelId) {
      setActiveChatView({ kind: "channel", id: visibleTextChannelId });
      if ((useUnreadStore.getState().byChannel[visibleTextChannelId] ?? 0) > 0) {
        useUnreadStore.getState().resetChannel(visibleTextChannelId);
        userAPI.markChannelRead(visibleTextChannelId).catch((err) => {
          console.warn("[read-state] mark channel read failed:", err);
        });
      }
    } else {
      setActiveChatView(null);
    }
    return () => setActiveChatView(null);
  }, [visibleDmId, visibleTextChannelId]);
}
