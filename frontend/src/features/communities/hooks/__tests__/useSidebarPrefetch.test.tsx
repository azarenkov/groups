import React from "react";
import { act, render } from "@testing-library/react";

import { PINNED_DM_RETRY_MS, useSidebarPrefetch } from "../useSidebarPrefetch";
import { useFriendshipStore } from "../../../social/store/friendshipStore";
import { useDmPreviewStore } from "../../../dm/store/dmPreviewStore";
import { useConversationPinsStore } from "../../store/conversationPinsStore";

const Harness: React.FC = () => {
  useSidebarPrefetch({
    query: "",
    hasSession: true,
    communities: [],
    channelsByCommunity: {},
    fetchChannels: () => undefined,
  });
  return null;
};

it("keeps retrying pages while a pinned DM is still missing", () => {
  jest.useFakeTimers();
  const loadMoreFriends = jest.fn(() => Promise.resolve());
  const loadMorePreviews = jest.fn(() => Promise.resolve());
  useFriendshipStore.setState({ friends: [], nextCursor: null, loadMore: loadMoreFriends } as never);
  useDmPreviewStore.setState({ byFriend: {}, nextCursor: "p2", loadMore: loadMorePreviews } as never);
  useConversationPinsStore.getState().applyRemote({ dms: ["far"] });
  render(<Harness />);
  expect(loadMorePreviews).toHaveBeenCalledTimes(1);
  act(() => {
    jest.advanceTimersByTime(PINNED_DM_RETRY_MS);
  });
  expect(loadMorePreviews).toHaveBeenCalledTimes(2);
  expect(loadMoreFriends).not.toHaveBeenCalled();
  act(() => {
    useDmPreviewStore.setState({ byFriend: { far: { peerName: "Far" } } } as never);
  });
  act(() => {
    jest.advanceTimersByTime(PINNED_DM_RETRY_MS * 10);
  });
  expect(loadMorePreviews).toHaveBeenCalledTimes(2);
  jest.useRealTimers();
});

it("gives a newly missing pinned DM a fresh retry budget", () => {
  jest.useFakeTimers();
  const loadMorePreviews = jest.fn(() => Promise.resolve());
  useFriendshipStore.setState({ friends: [], nextCursor: null, loadMore: jest.fn() } as never);
  useDmPreviewStore.setState({ byFriend: {}, nextCursor: "p2", loadMore: loadMorePreviews } as never);
  useConversationPinsStore.getState().applyRemote({ dms: ["one"] });
  render(<Harness />);
  for (let i = 0; i < 10; i += 1) {
    act(() => {
      jest.advanceTimersByTime(PINNED_DM_RETRY_MS * 64);
    });
  }
  const exhausted = loadMorePreviews.mock.calls.length;
  act(() => {
    useDmPreviewStore.setState({ byFriend: { one: { peerName: "One" } } } as never);
  });
  act(() => {
    useConversationPinsStore.getState().applyRemote({ dms: ["one", "two"] });
  });
  for (let i = 0; i < 3; i += 1) {
    act(() => {
      jest.advanceTimersByTime(PINNED_DM_RETRY_MS * 4);
    });
  }
  expect(loadMorePreviews.mock.calls.length).toBeGreaterThan(exhausted + 1);
  jest.useRealTimers();
});
