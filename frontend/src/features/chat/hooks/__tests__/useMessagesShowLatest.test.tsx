import React from "react";
import { act, render, waitFor } from "@testing-library/react";

import type { ChannelMessage } from "../../../../shared/types/channel";
import { clearMessageCache, saveConversation } from "../../lib/messageCache";
import type { Messenger } from "../useMessenger";
import { useMessages } from "../useMessages";

const msg = (id: string, createdAt: string): ChannelMessage => ({
  id,
  channelId: "c",
  authorId: "u2",
  content: id,
  createdAt,
  editedAt: null,
});

const latestPage = [msg("m9", "2026-10-01T10:09:00Z")];
const target = msg("m1", "2026-10-01T10:01:00Z");
const pending: Array<(value: unknown) => void> = [];
const messenger = {
  listMessages: (query: { before?: string; after?: string }) =>
    query.before || query.after
      ? new Promise((resolve) => pending.push(resolve))
      : Promise.resolve({ messages: latestPage, partition: "messaging:latest", seq: 1 }),
  getMessage: async (id: string) => {
    if (id === "gone") throw new Error("gone");
    return target;
  },
} as unknown as Messenger;

let latest: ReturnType<typeof useMessages> | null = null;
const Harness: React.FC = () => {
  latest = useMessages({
    messenger,
    convId: "latest",
    isText: true,
    isDm: false,
    nameSyncTick: 0,
    reseedChannelReads: () => undefined,
    currentUserId: "me",
  });
  return <div ref={latest.bodyRef} />;
};

beforeEach(() => {
  clearMessageCache();
  pending.length = 0;
});

it("drops an in-flight jump and its loader when the regular view is requested", async () => {
  render(<Harness />);
  await waitFor(() => expect(latest!.showInitialLoader).toBe(false));
  let jump: Promise<unknown> | undefined;
  act(() => {
    jump = latest!.jumpToMessageId("m1");
  });
  await waitFor(() => expect(latest!.showInitialLoader).toBe(true));
  act(() => latest!.showLatest());
  expect(latest!.showInitialLoader).toBe(false);
  await act(async () => {
    pending.forEach((resolve) => resolve({ messages: [] }));
    await jump;
  });
  expect(await jump).toBe("stale");
  expect(latest!.messages.map((m) => m.id)).toEqual(["m9"]);
  expect(latest!.showInitialLoader).toBe(false);
});

it("does not restore the highlight of a loaded message after the regular view is requested", async () => {
  render(<Harness />);
  await waitFor(() => expect(latest!.messages.map((m) => m.id)).toEqual(["m9"]));
  const frames: FrameRequestCallback[] = [];
  const raf = jest.spyOn(window, "requestAnimationFrame").mockImplementation((cb) => {
    frames.push(cb);
    return frames.length;
  });
  try {
    await act(async () => {
      await latest!.jumpToMessageId("m9");
    });
    act(() => latest!.showLatest());
    act(() => frames.forEach((cb) => cb(0)));
    expect(latest!.highlightedMessageId).toBeNull();
  } finally {
    raf.mockRestore();
  }
});

it("does not start a jump that was cancelled while the cached chat was revalidating", async () => {
  const revalidations: Array<(value: unknown) => void> = [];
  const contextRequests = jest.fn();
  const revalidating = {
    listMessages: (query: { before?: string; after?: string }) => {
      if (query.before || query.after) {
        contextRequests();
        return new Promise(() => {});
      }
      return new Promise((resolve) => revalidations.push(resolve));
    },
    getMessage: async () => target,
  } as unknown as Messenger;
  saveConversation("cached", { messages: latestPage, hasMore: false, scroll: { atBottom: true, anchorId: null, offset: 0 } });
  let current: ReturnType<typeof useMessages> | null = null;
  const Cached: React.FC = () => {
    current = useMessages({
      messenger: revalidating,
      convId: "cached",
      isText: true,
      isDm: false,
      nameSyncTick: 0,
      reseedChannelReads: () => undefined,
      currentUserId: "me",
    });
    return <div ref={current.bodyRef} />;
  };
  render(<Cached />);
  let jump: Promise<unknown> | undefined;
  await act(async () => {
    jump = current!.jumpToMessageId("m1");
  });
  act(() => current!.showLatest());
  await act(async () => {
    revalidations.forEach((resolve) => resolve({ messages: latestPage, partition: "messaging:cached", seq: 1 }));
    await jump;
  });
  expect(await jump).toBe("stale");
  expect(contextRequests).not.toHaveBeenCalled();
  expect(current!.showInitialLoader).toBe(false);
});

it("lets a newer jump in while an older jump is loading its context", async () => {
  render(<Harness />);
  await waitFor(() => expect(latest!.historyLoading).toBe(false));
  let first: Promise<unknown> | undefined;
  act(() => {
    first = latest!.jumpToMessageId("m1");
  });
  await waitFor(() => expect(latest!.showInitialLoader).toBe(true));
  expect(latest!.historyLoading).toBe(false);
  let second: Promise<unknown> | undefined;
  act(() => {
    second = latest!.jumpToMessageId("m1");
  });
  await act(async () => {
    pending.forEach((resolve) => resolve({ messages: [] }));
    await first;
    await waitFor(() => expect(pending.length).toBe(4));
    pending.slice(2).forEach((resolve) => resolve({ messages: [] }));
    await second;
  });
  expect(await first).toBe("stale");
  expect(await second).toBe("shown");
  expect(latest!.showInitialLoader).toBe(false);
});

it("releases an older jump's loader when a newer jump targets a loaded message", async () => {
  render(<Harness />);
  await waitFor(() => expect(latest!.historyLoading).toBe(false));
  act(() => {
    void latest!.jumpToMessageId("m1");
  });
  await waitFor(() => expect(latest!.showInitialLoader).toBe(true));
  let second: Promise<unknown> | undefined;
  await act(async () => {
    second = latest!.jumpToMessageId("m9");
    await second;
  });
  expect(await second).toBe("shown");
  expect(latest!.showInitialLoader).toBe(false);
});

it("releases an older jump's loader when a newer jump fails to find its message", async () => {
  render(<Harness />);
  await waitFor(() => expect(latest!.historyLoading).toBe(false));
  act(() => {
    void latest!.jumpToMessageId("m1");
  });
  await waitFor(() => expect(latest!.showInitialLoader).toBe(true));
  let second: Promise<unknown> | undefined;
  await act(async () => {
    second = latest!.jumpToMessageId("gone");
    await second;
  });
  expect(await second).toBe("failed");
  expect(latest!.showInitialLoader).toBe(false);
});
