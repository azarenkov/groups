import React from "react";
import { useHotkey } from "../../../settings/lib/hotkeys";
import type { Channel } from "../../../../shared/types/channel";

export type SidebarScreen = { tab: "dms" | "communities"; community: string | null };

export function useSidebarNavigation(params: {
  selectedChannelId: string | null;
  selectedDmId: string | null;
  channelsByCommunity: Record<string, Channel[]>;
}) {
  const { selectedChannelId, selectedDmId, channelsByCommunity } = params;
  const initialScreen: SidebarScreen = {
    tab: selectedChannelId ? "communities" : "dms",
    community: null,
  };
  const [screen, setScreen] = React.useState<SidebarScreen>(initialScreen);
  const historyRef = React.useRef<SidebarScreen[]>([initialScreen]);
  const historyIdxRef = React.useRef(0);
  const [navState, setNavState] = React.useState({ back: false, forward: false });
  const tab = screen.tab;
  const openCommunityId = screen.tab === "communities" ? screen.community : null;

  const syncNavState = React.useCallback(() => {
    setNavState({
      back: historyIdxRef.current > 0,
      forward: historyIdxRef.current < historyRef.current.length - 1,
    });
  }, []);
  const navigateTo = React.useCallback(
    (next: SidebarScreen) => {
      const cur = historyRef.current[historyIdxRef.current];
      if (cur && cur.tab === next.tab && cur.community === next.community) return;
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
    (next: "dms" | "communities") => {
      const latest = historyRef.current[historyIdxRef.current];
      navigateTo({
        tab: next,
        community:
          next === "communities"
            ? latest?.tab === "communities"
              ? latest.community
              : openCommunityId
            : null,
      });
    },
    [navigateTo, openCommunityId],
  );
  const setOpenCommunityId = React.useCallback(
    (id: string | null) => navigateTo({ tab: "communities", community: id }),
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
      setTab("communities");
    }
    prevSelection.current = { dm: selectedDmId, channel: selectedChannelId };
  }, [selectedDmId, selectedChannelId, setTab]);
  const channelsByCommunityRef = React.useRef(channelsByCommunity);
  channelsByCommunityRef.current = channelsByCommunity;
  React.useEffect(() => {
    if (!selectedChannelId) return;
    for (const [sid, list] of Object.entries(channelsByCommunityRef.current)) {
      if (list.some((c) => c.id === selectedChannelId)) {
        setOpenCommunityId(sid);
        return;
      }
    }
  }, [selectedChannelId, setOpenCommunityId]);

  return { tab, openCommunityId, navState, goHistory, setTab, setOpenCommunityId };
}
