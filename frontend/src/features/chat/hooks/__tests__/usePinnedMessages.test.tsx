import React from "react";
import { act, render, waitFor } from "@testing-library/react";

import { pinsAPI, type PinnedMessage } from "../../api/pins";
import type { SharedScope } from "../../api/shared";
import { PIN_REFRESH_DEBOUNCE_MS, pinEventMatches, usePinnedMessages } from "../usePinnedMessages";

jest.mock("../../api/pins", () => ({
  pinsAPI: { list: jest.fn(), pin: jest.fn(), unpin: jest.fn() },
}));

const listeners: ((env: { type: string; data: unknown }) => void)[] = [];
const openListeners: (() => void)[] = [];
jest.mock("../../../../shared/ws/websocket", () => ({
  ws: {
    onOpen: (fn: () => void) => {
      openListeners.push(fn);
      return () => openListeners.splice(openListeners.indexOf(fn), 1);
    },
    on: (fn: (env: { type: string; data: unknown }) => void) => {
      listeners.push(fn);
      return () => listeners.splice(listeners.indexOf(fn), 1);
    },
  },
}));

const emit = (type: string, data: unknown) =>
  act(() => {
    [...listeners].forEach((fn) => fn({ type, data }));
  });

const list = pinsAPI.list as jest.Mock;
const unpin = pinsAPI.unpin as jest.Mock;

const pin = (id: string): PinnedMessage => ({
  messageId: id,
  authorId: "u2",
  authorName: "Bob",
  content: id,
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

beforeEach(() => {
  list.mockReset();
  unpin.mockReset();
  listeners.length = 0;
  openListeners.length = 0;
  latest = null;
});

it("matches pin events to the open conversation only", () => {
  expect(pinEventMatches({ channelId: "c1" }, { kind: "channel", communityId: "s", channelId: "c1" })).toBe(true);
  expect(pinEventMatches({ channelId: "c2" }, { kind: "channel", communityId: "s", channelId: "c1" })).toBe(false);
  expect(pinEventMatches({ participants: ["u1", "u2"] }, dm, "u1")).toBe(true);
  expect(pinEventMatches({ participants: ["u1", "u3"] }, dm, "u1")).toBe(false);
});

it("loads pins, refreshes on pin events and drops deleted messages", async () => {
  jest.useFakeTimers();
  list.mockResolvedValueOnce([pin("a"), pin("b")]).mockResolvedValueOnce([pin("c"), pin("a"), pin("b")]);
  render(<Harness scope={dm} />);
  await act(async () => {
    await Promise.resolve();
  });
  expect(latest!.pins.map((p) => p.messageId)).toEqual(["a", "b"]);
  emit("message.pin_changed", { messageId: "c", participants: ["u1", "u2"], pinned: true });
  await act(async () => {
    jest.advanceTimersByTime(PIN_REFRESH_DEBOUNCE_MS);
    await Promise.resolve();
  });
  expect(latest!.pins.map((p) => p.messageId)).toEqual(["c", "a", "b"]);
  emit("dm_message.deleted", { id: "a", senderId: "u2", recipientId: "u1" });
  emit("message.hidden", { messageId: "b" });
  expect(latest!.pins.map((p) => p.messageId)).toEqual(["c"]);
  jest.useRealTimers();
});

it("ignores a late response for the previous conversation", async () => {
  let resolveOld: (items: PinnedMessage[]) => void = () => undefined;
  list.mockImplementationOnce(() => new Promise((resolve) => (resolveOld = resolve))).mockResolvedValueOnce([pin("new")]);
  const { rerender } = render(<Harness scope={dm} />);
  rerender(<Harness scope={{ kind: "dm", friendId: "u3" }} />);
  await waitFor(() => expect(latest!.pins.map((p) => p.messageId)).toEqual(["new"]));
  await act(async () => resolveOld([pin("old")]));
  expect(latest!.pins.map((p) => p.messageId)).toEqual(["new"]);
});

it("removes a pin right away when unpinning", async () => {
  list.mockResolvedValue([pin("a")]);
  unpin.mockResolvedValue(undefined);
  render(<Harness scope={dm} />);
  await waitFor(() => expect(latest!.pins).toHaveLength(1));
  list.mockResolvedValue([]);
  await act(async () => {
    await latest!.setPinned("a", false);
  });
  expect(unpin).toHaveBeenCalledWith(dm, "a");
  expect(latest!.pins).toHaveLength(0);
});

it("applies only the newest refresh and keeps deleted pins gone", async () => {
  jest.useFakeTimers();
  const resolvers: ((items: PinnedMessage[]) => void)[] = [];
  list.mockResolvedValueOnce([pin("a"), pin("b")]);
  render(<Harness scope={dm} />);
  await act(async () => {
    await Promise.resolve();
  });
  list.mockImplementation(() => new Promise((resolve) => resolvers.push(resolve)));
  emit("message.pin_changed", { participants: ["u1", "u2"] });
  await act(async () => {
    jest.advanceTimersByTime(PIN_REFRESH_DEBOUNCE_MS);
  });
  emit("dm_message.deleted", { id: "a", senderId: "u2", recipientId: "u1" });
  emit("message.pin_changed", { participants: ["u1", "u2"] });
  await act(async () => {
    jest.advanceTimersByTime(PIN_REFRESH_DEBOUNCE_MS);
  });
  expect(resolvers).toHaveLength(2);
  await act(async () => resolvers[1]([pin("b")]));
  await act(async () => resolvers[0]([pin("a"), pin("b")]));
  expect(latest!.pins.map((p) => p.messageId)).toEqual(["b"]);
  jest.useRealTimers();
});

it("never shows the previous conversation's pins after switching", async () => {
  list.mockResolvedValueOnce([pin("a")]).mockImplementationOnce(() => new Promise(() => undefined));
  const seen: string[][] = [];
  const Probe: React.FC<{ scope: SharedScope }> = ({ scope }) => {
    const result = usePinnedMessages(scope, "u1");
    seen.push(result.pins.map((p) => p.messageId));
    return null;
  };
  const { rerender } = render(<Probe scope={dm} />);
  await waitFor(() => expect(seen[seen.length - 1]).toEqual(["a"]));
  const before = seen.length;
  rerender(<Probe scope={{ kind: "dm", friendId: "u3" }} />);
  expect(seen.slice(before).every((ids) => ids.length === 0)).toBe(true);
});

it("reloads pins after the socket reconnects", async () => {
  list.mockResolvedValueOnce([pin("a")]).mockResolvedValueOnce([pin("b"), pin("a")]);
  render(<Harness scope={dm} />);
  await waitFor(() => expect(latest!.pins.map((p) => p.messageId)).toEqual(["a"]));
  act(() => {
    openListeners.forEach((fn) => fn());
  });
  await waitFor(() => expect(latest!.pins.map((p) => p.messageId)).toEqual(["b", "a"]));
});

it("refreshes when a message is edited while the first load is still running", async () => {
  jest.useFakeTimers();
  let resolveFirst: (items: PinnedMessage[]) => void = () => undefined;
  list.mockImplementationOnce(() => new Promise((resolve) => (resolveFirst = resolve))).mockResolvedValueOnce([
    { ...pin("a"), content: "edited" },
  ]);
  render(<Harness scope={dm} />);
  emit("dm_message.edited", { id: "a", senderId: "u2", recipientId: "u1", newContent: "edited" });
  await act(async () => resolveFirst([pin("a")]));
  await act(async () => {
    jest.advanceTimersByTime(PIN_REFRESH_DEBOUNCE_MS);
    await Promise.resolve();
  });
  expect(latest!.pins[0].content).toBe("edited");
  jest.useRealTimers();
});

it("does not store a late answer under the newly opened chat", async () => {
  let resolveOld: (items: PinnedMessage[]) => void = () => undefined;
  list.mockImplementationOnce(() => new Promise((resolve) => (resolveOld = resolve))).mockImplementationOnce(() => new Promise(() => undefined));
  const seen: string[][] = [];
  const Probe: React.FC<{ scope: SharedScope }> = ({ scope }) => {
    const result = usePinnedMessages(scope, "u1");
    if (scope.kind === "dm" && scope.friendId === "u3") {
      resolveOld([pin("old")]);
    }
    seen.push(result.pins.map((p) => p.messageId));
    return null;
  };
  const { rerender } = render(<Probe scope={dm} />);
  rerender(<Probe scope={{ kind: "dm", friendId: "u3" }} />);
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
  rerender(<Probe scope={{ kind: "dm", friendId: "u3" }} />);
  expect(seen.every((ids) => !ids.includes("old"))).toBe(true);
});

it("keeps an older successful load when a newer overlapping one fails", async () => {
  let resolveFirst: (items: PinnedMessage[]) => void = () => undefined;
  let rejectSecond: (e: unknown) => void = () => undefined;
  list
    .mockImplementationOnce(() => new Promise((resolve) => (resolveFirst = resolve)))
    .mockImplementationOnce(() => new Promise((_resolve, reject) => (rejectSecond = reject)));
  render(<Harness scope={dm} />);
  act(() => {
    openListeners.forEach((fn) => fn());
  });
  await act(async () => rejectSecond(new Error("offline")));
  await act(async () => resolveFirst([pin("a")]));
  expect(latest!.pins.map((p) => p.messageId)).toEqual(["a"]);
});
