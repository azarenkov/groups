import React from "react";
import { act, render } from "@testing-library/react";

import { useSelection } from "../useSelection";
import { useCallStore } from "../../../voice/store/callStore";
import type { useVoiceSession } from "../../../voice/hooks/useVoiceSession";
import type { Channel } from "../../../../shared/types/channel";

const stage = { id: "voice-1", kind: "voice" } as Channel;
const setSelectedCommunity = jest.fn();
const setSelectedChannel = jest.fn();
let selection: ReturnType<typeof useSelection> | null = null;

const Harness: React.FC = () => {
  selection = useSelection({
    hasSession: true,
    communities: [],
    channelsByCommunity: {},
    selectedCommunityId: null,
    selectedChannelId: null,
    setSelectedCommunity,
    setSelectedChannel,
    fetchCommunities: async () => undefined,
    fetchChannels: () => undefined,
    voice: { joinedChannel: { communityId: "community-1", channel: stage } } as unknown as ReturnType<typeof useVoiceSession>,
  });
  return null;
};

it("counts opening the joined voice stage as an explicit selection", () => {
  render(<Harness />);
  const before = selection?.selectionSeq;
  useCallStore.setState({ focused: true });
  act(() => selection?.handleOpenJoinedStage());
  expect(selection?.selectionSeq).toBe((before ?? 0) + 1);
  expect(setSelectedCommunity).toHaveBeenLastCalledWith("community-1");
  expect(setSelectedChannel).toHaveBeenLastCalledWith("voice-1");
  expect(selection?.selectedDm).toBeNull();
  expect(useCallStore.getState().focused).toBe(false);
});
