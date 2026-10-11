import React from "react";
import { act, renderHook } from "@testing-library/react";

import { useMessageComposer } from "../useMessageComposer";
import { ws } from "../../../../shared/ws/websocket";
import { useAuthStore } from "../../../auth/store/authStore";

const render = (friendId: string) =>
  renderHook(() =>
    useMessageComposer({
      messenger: null,
      dm: { friendId, name: "Bob", initials: "B", color: "#333", online: false },
      isText: true,
      convId: friendId,
      attachments: {} as never,
      setMessages: jest.fn(),
      currentChannelIdRef: React.createRef() as never,
      refreshHistory: jest.fn(),
    }),
  );

const reply = { id: "m1", content: "secret", hasAttachments: false, deleted: false };
const clearedWith = (peer: string, at: string) =>
  ws.deliver({ type: "dm_conversation.cleared", data: { participants: ["me", peer], scope: "me", clearedAt: at } });

beforeEach(() => {
  useAuthStore.setState({ user: { id: "me", name: "Me" } } as never);
});

it("drops the reply target when its conversation is cleared", () => {
  const { result } = render("peer");
  act(() => result.current.setReplyTarget(reply));
  act(() => clearedWith("other", "2026-10-11T10:00:00Z"));
  expect(result.current.replyTarget).toEqual(reply);
  act(() => clearedWith("peer", "2026-10-11T10:00:01Z"));
  expect(result.current.replyTarget).toBeNull();
});
