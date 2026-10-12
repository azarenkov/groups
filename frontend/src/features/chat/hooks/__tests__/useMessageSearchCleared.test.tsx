import { act, renderHook, waitFor } from "@testing-library/react";

import { useMessageSearch } from "../useMessageSearch";
import { ws } from "../../../../shared/ws/websocket";
import type { Messenger } from "../useMessenger";
import type { ChannelMessage } from "../../../../shared/types/channel";

const msg = (id: string, createdAt: string): ChannelMessage =>
  ({ id, channelId: "", authorId: "peer", content: id, createdAt, editedAt: null }) as ChannelMessage;

it("drops and refetches DM search results when the conversation is cleared", async () => {
  const search = jest
    .fn()
    .mockResolvedValueOnce({ messages: [msg("old", "2026-10-11T09:00:00Z")] })
    .mockResolvedValueOnce({ messages: [] });
  const messenger = { searchMessages: search } as unknown as Messenger;
  const { result } = renderHook(() =>
    useMessageSearch({ messenger, convId: "peer", nameSyncTick: 0, dmFriendId: "peer", currentUserId: "me", onClose: jest.fn() }),
  );
  act(() => result.current.toggleSearch());
  act(() => result.current.setSearchInput("old"));
  await waitFor(() => expect(result.current.searchResults).toHaveLength(1));
  act(() =>
    ws.deliver({
      type: "dm_conversation.cleared",
      data: { participants: ["me", "peer"], scope: "me", clearedAt: "2026-10-11T10:00:00Z" },
    }),
  );
  expect(result.current.searchResults).toHaveLength(0);
  await waitFor(() => expect(search).toHaveBeenCalledTimes(2));
});
