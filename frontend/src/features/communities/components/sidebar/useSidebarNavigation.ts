import React from "react";
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
  const tab = screen.tab;
  const openCommunityId = screen.tab === "communities" ? screen.community : null;

  const setTab = React.useCallback(
    (next: "dms" | "communities") =>
      setScreen((latest) => ({
        tab: next,
        community: next === "communities" ? (latest.tab === "communities" ? latest.community : null) : null,
      })),
    [],
  );
  const setOpenCommunityId = React.useCallback(
    (id: string | null) => setScreen({ tab: "communities", community: id }),
    [],
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

  return { tab, openCommunityId, setTab, setOpenCommunityId };
}
