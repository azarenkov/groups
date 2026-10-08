import React from "react";
import { act, render, waitFor } from "@testing-library/react";

import { pinsAPI, type PinnedMessage } from "../../api/pins";
import type { SharedScope } from "../../api/shared";
import { clearPinTombstones, pinEventMatches, usePinnedMessages } from "../usePinnedMessages";

jest.mock("../../api/pins", () => ({
  pinsAPI: { list: jest.fn(), pin: jest.fn(), unpin: jest.fn() },
}));

const listeners: ((env: { type: string; data: unknown }) => void)[] = [];
jest.mock("../../../../shared/ws/websocket", () => ({
  ws: {
    on: (fn: (env: { type: string; data: unknown }) => void) => {
      listeners.push(fn);
      return () => listeners.splice(listeners.indexOf(fn), 1);
    },
    onOpen: () => () => undefined,
  },
}));

const emit = (type: string, data: unknown) =>
  act(() => {
    [...listeners].forEach((fn) => fn({ type, data }));
  });

const list = pinsAPI.list as jest.Mock;
const unpin = pinsAPI.unpin as jest.Mock;

const pin = (id: string, content = id): PinnedMessage => ({
  messageId: id,
  authorId: "u2",
  authorName: "Bob",
  content,
  hasAttachments: false,
  createdAt: "2026-10-01T10:00:00Z",
  pinnedBy: "u2",
  pinnedAt: "2026-10-01T10:00:00Z",
});

let latest: ReturnType<typeof usePinnedMessages> | null = null;
const Harness: React.FC<{ scope: SharedScope | null }> = ({ scope }) => {
  latest = usePinnedMessages(scope, "u1");
  return null;
};

const dm: SharedScope = { kind: "dm", friendId: "u2" };
const ids = () => latest!.pins.map((p) => p.messageId);

beforeEach(() => {
  list.mockReset();
  unpin.mockReset();
  listeners.length = 0;
  latest = null;
  clearPinTombstones();
});

it("matches pin events to the open conversation only", () => {
  expect(pinEventMatches({ channelId: "c1" }, { kind: "channel", communityId: "s", channelId: "c1" })).toBe(true);
  expect(pinEventMatches({ channelId: "c2" }, { kind: "channel", communityId: "s", channelId: "c1" })).toBe(false);
  expect(pinEventMatches({ participants: ["u1", "u2"] }, dm, "u1")).toBe(true);
  expect(pinEventMatches({ participants: ["u1", "u3"] }, dm, "u1")).toBe(false);
});

it("loads pins, refreshes on pin events and drops deleted or hidden messages", async () => {
  list.mockResolvedValueOnce([pin("a"), pin("b")]).mockResolvedValueOnce([pin("c"), pin("a"), pin("b")]);
  render(<Harness scope={dm} />);
  await waitFor(() => expect(ids()).toEqual(["a", "b"]));
  emit("message.pin_changed", { messageId: "c", participants: ["u1", "u2"], pinned: true });
  await waitFor(() => expect(ids()).toEqual(["c", "a", "b"]));
  emit("dm_message.deleted", { id: "a", senderId: "u2", recipientId: "u1" });
  emit("message.hidden", { messageId: "b" });
  await waitFor(() => expect(ids()).toEqual(["c"]));
});

it("ignores a late response for the previous conversation", async () => {
  let resolveOld: (items: PinnedMessage[]) => void = () => undefined;
  list.mockImplementationOnce(() => new Promise((resolve) => (resolveOld = resolve))).mockResolvedValueOnce([pin("new")]);
  const { rerender } = render(<Harness scope={dm} />);
  rerender(<Harness scope={{ kind: "dm", friendId: "u3" }} />);
  await waitFor(() => expect(ids()).toEqual(["new"]));
  await act(async () => resolveOld([pin("old")]));
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  expect(ids()).toEqual(["new"]);
});

it("shows the pins of a chat opened before right away when coming back", async () => {
  list.mockResolvedValueOnce([pin("a")]).mockResolvedValueOnce([pin("x")]).mockImplementation(() => new Promise(() => undefined));
  const { rerender } = render(<Harness scope={dm} />);
  await waitFor(() => expect(ids()).toEqual(["a"]));
  rerender(<Harness scope={{ kind: "dm", friendId: "u3" }} />);
  await waitFor(() => expect(ids()).toEqual(["x"]));
  rerender(<Harness scope={dm} />);
  expect(ids()).toEqual(["a"]);
});

it("removes a pin right away when unpinning", async () => {
  list.mockResolvedValueOnce([pin("a")]).mockImplementation(() => new Promise(() => undefined));
  unpin.mockResolvedValue(undefined);
  render(<Harness scope={dm} />);
  await waitFor(() => expect(ids()).toEqual(["a"]));
  await act(async () => {
    void latest!.setPinned("a", false);
    await Promise.resolve();
    await Promise.resolve();
  });
  expect(unpin).toHaveBeenCalledWith(dm, "a");
  await waitFor(() => expect(latest!.pins).toHaveLength(0));
});

it("applies only the newest refresh and keeps deleted pins gone", async () => {
  const resolvers: ((items: PinnedMessage[]) => void)[] = [];
  list.mockResolvedValueOnce([pin("a"), pin("b")]);
  render(<Harness scope={dm} />);
  await waitFor(() => expect(ids()).toEqual(["a", "b"]));
  list.mockImplementation(() => new Promise((resolve) => resolvers.push(resolve)));
  emit("message.pin_changed", { participants: ["u1", "u2"] });
  emit("dm_message.deleted", { id: "a", senderId: "u2", recipientId: "u1" });
  emit("message.pin_changed", { participants: ["u1", "u2"] });
  await waitFor(() => expect(resolvers).toHaveLength(2));
  await act(async () => resolvers[1]([pin("b")]));
  await act(async () => resolvers[0]([pin("a"), pin("b")]));
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  expect(ids()).toEqual(["b"]);
});

it("refreshes when a message is edited while the first load is still running", async () => {
  let resolveFirst: (items: PinnedMessage[]) => void = () => undefined;
  list
    .mockImplementationOnce(() => new Promise((resolve) => (resolveFirst = resolve)))
    .mockResolvedValueOnce([pin("a", "edited")]);
  render(<Harness scope={dm} />);
  emit("dm_message.edited", { id: "a", senderId: "u2", recipientId: "u1", newContent: "edited" });
  await act(async () => resolveFirst([pin("a")]));
  await waitFor(() => expect(latest!.pins[0]?.content).toBe("edited"));
});

it("shows a pin again when it is pinned back from elsewhere", async () => {
  list.mockResolvedValueOnce([pin("a")]);
  unpin.mockResolvedValue(undefined);
  render(<Harness scope={dm} />);
  await waitFor(() => expect(ids()).toEqual(["a"]));
  list.mockResolvedValueOnce([]);
  await act(async () => {
    await latest!.setPinned("a", false);
  });
  await waitFor(() => expect(ids()).toEqual([]));
  list.mockResolvedValueOnce([pin("a")]);
  emit("message.pin_changed", { messageId: "a", participants: ["u1", "u2"], pinned: true });
  await waitFor(() => expect(ids()).toEqual(["a"]));
});
