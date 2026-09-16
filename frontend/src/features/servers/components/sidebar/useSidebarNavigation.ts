import React from "react";
import { useHotkey } from "../../../settings/lib/hotkeys";
import type { Channel } from "../../../../shared/types/channel";

export type SidebarScreen = { tab: "dms" | "servers"; server: string | null };

export function useSidebarNavigation(params: {
  selectedChannelId: string | null;
  selectedDmId: string | null;
  channelsByServer: Record<string, Channel[]>;
}) {
  const { selectedChannelId, selectedDmId, channelsByServer } = params;
  const initialScreen: SidebarScreen = {
    tab: selectedChannelId ? "servers" : "dms",
    server: null,
  };
  const [screen, setScreen] = React.useState<SidebarScreen>(initialScreen);
  const historyRef = React.useRef<SidebarScreen[]>([initialScreen]);
  const historyIdxRef = React.useRef(0);
  const [navState, setNavState] = React.useState({ back: false, forward: false });
  const tab = screen.tab;
  const openServerId = screen.tab === "servers" ? screen.server : null;

  const syncNavState = React.useCallback(() => {
    setNavState({
      back: historyIdxRef.current > 0,
      forward: historyIdxRef.current < historyRef.current.length - 1,
    });
  }, []);
  const navigateTo = React.useCallback(
    (next: SidebarScreen) => {
      const cur = historyRef.current[historyIdxRef.current];
      if (cur && cur.tab === next.tab && cur.server === next.server) return;
      const hist = historyRef.current.slice(0, historyIdxRef.current + 1);
      hist.push(next);
      if (hist.length > 50) hist.shift();
      historyRef.current = hist;
      historyIdxRef.current = hist.length - 1;
      setScreen(next);
      syncNavState();
    },
    [syncNavState],
  );
  const goHistory = React.useCallback(
    (delta: -1 | 1) => {
      const idx = historyIdxRef.current + delta;
      if (idx < 0 || idx >= historyRef.current.length) return;
      historyIdxRef.current = idx;
      setScreen(historyRef.current[idx]);
      syncNavState();
    },
    [syncNavState],
  );
  useHotkey("historyBack", () => goHistory(-1));
  useHotkey("historyForward", () => goHistory(1));
  const setTab = React.useCallback(
    (next: "dms" | "servers") => {
      const latest = historyRef.current[historyIdxRef.current];
      navigateTo({
        tab: next,
        server:
          next === "servers"
            ? latest?.tab === "servers"
              ? latest.server
              : openServerId
            : null,
      });
    },
    [navigateTo, openServerId],
  );
  const setOpenServerId = React.useCallback(
    (id: string | null) => navigateTo({ tab: "servers", server: id }),
    [navigateTo],
  );
  const prevSelection = React.useRef({
    dm: selectedDmId,
    channel: selectedChannelId,
  });
  React.useEffect(() => {
    const prev = prevSelection.current;
    if (selectedDmId && selectedDmId !== prev.dm) {
      setTab("dms");
    } else if (selectedChannelId && selectedChannelId !== prev.channel) {
      setTab("servers");
    }
    prevSelection.current = { dm: selectedDmId, channel: selectedChannelId };
  }, [selectedDmId, selectedChannelId, setTab]);
  const channelsByServerRef = React.useRef(channelsByServer);
  channelsByServerRef.current = channelsByServer;
  React.useEffect(() => {
    if (!selectedChannelId) return;
    for (const [sid, list] of Object.entries(channelsByServerRef.current)) {
      if (list.some((c) => c.id === selectedChannelId)) {
        setOpenServerId(sid);
        return;
      }
    }
  }, [selectedChannelId, setOpenServerId]);

  return { tab, openServerId, navState, goHistory, setTab, setOpenServerId };
}
