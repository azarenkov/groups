import React from "react";
import { act, render } from "@testing-library/react";

import { useSelection } from "../useSelection";
import { userAPI } from "../../../social/api/user";
import { useCallStore } from "../../../voice/store/callStore";
import type { useVoiceSession } from "../../../voice/hooks/useVoiceSession";
import type { Channel } from "../../../../shared/types/channel";
import type { DirectConversation } from "../../../dm/types";

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

beforeEach(() => {
  jest.spyOn(userAPI, "markDmRead").mockResolvedValue(undefined as never);
});

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

it("shows a linked chat without counting it as a selection by the user", () => {
  render(<Harness />);
  const before = selection?.selectionSeq;
  const dm = { id: "peer-1", name: "Alice" } as DirectConversation;
  act(() => selection?.showDm(dm));
  expect(selection?.selectionSeq).toBe(before);
  expect(selection?.selectedDm).toBe(dm);
  act(() => selection?.handleSelectDm(dm));
  expect(selection?.selectionSeq).toBe((before ?? 0) + 1);
});
