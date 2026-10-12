import React from "react";
import { act, render, waitFor } from "@testing-library/react";

import type { ChannelMessage } from "../../../../shared/types/channel";
import { clearMessageCache, hasCachedConversation } from "../../lib/messageCache";
import { ApiError } from "../../../../shared/lib/apiError";
import type { Messenger } from "../useMessenger";
import { useMessages } from "../useMessages";
import { PAGE_LIMIT } from "../../constants";

const msg = (id: string, minute: number): ChannelMessage => ({
  id,
  channelId: "c",
  authorId: "u2",
  content: id,
  createdAt: `2026-10-01T10:${String(minute).padStart(2, "0")}:00Z`,
  editedAt: null,
});

const pages: Record<string, ChannelMessage[]> = {};
const pending: Record<string, ((messages: ChannelMessage[]) => void) | undefined> = {};
const failing: Record<string, ((err: unknown) => void) | undefined> = {};

const olderCalls: string[] = [];
const callCount: Record<string, number> = {};
const messengerFor = (convId: string): Messenger =>
  ({
    listMessages: (opts: { before?: string }) => {
      callCount[convId] = (callCount[convId] ?? 0) + 1;
      if (opts.before) olderCalls.push(`${convId}:${opts.before}`);
      return new Promise((resolve, reject) => {
        pending[convId] = (messages) => resolve({ messages: [...messages].reverse() });
        failing[convId] = reject;
      });
    },
  }) as unknown as Messenger;

let latest: ReturnType<typeof useMessages> | null = null;
const Harness: React.FC<{ convId: string; tick?: number }> = ({ convId, tick = 0 }) => {
  const messenger = React.useMemo(() => messengerFor(convId), [convId]);
  latest = useMessages({
    messenger,
    convId,
    isText: true,
    isDm: false,
    nameSyncTick: tick,
    reseedChannelReads: () => undefined,
    currentUserId: "me",
  });
  return <div ref={latest.bodyRef} />;
};

beforeEach(() => {
  clearMessageCache();
  latest = null;
});

const answer = async (convId: string, messages: ChannelMessage[]) => {
  pages[convId] = messages;
  await act(async () => {
    pending[convId]?.(messages);
    await Promise.resolve();
  });
};

it("shows a recently opened chat from cache and refreshes it quietly", async () => {
  const { rerender } = render(<Harness convId="a" />);
  expect(latest!.showInitialLoader).toBe(true);
  await answer("a", [msg("a1", 1), msg("a2", 2)]);
  expect(latest!.messages.map((m) => m.id)).toEqual(["a1", "a2"]);

  rerender(<Harness convId="b" />);
  expect(latest!.showInitialLoader).toBe(true);
  expect(latest!.messages).toEqual([]);
  await answer("b", [msg("b1", 1)]);
  expect(hasCachedConversation("a")).toBe(true);

  rerender(<Harness convId="a" />);
  expect(latest!.showInitialLoader).toBe(false);
  expect(latest!.messages.map((m) => m.id)).toEqual(["a1", "a2"]);
  await answer("a", [msg("a2", 2), msg("a3", 3)]);
  await waitFor(() => expect(latest!.messages.map((m) => m.id)).toEqual(["a2", "a3"]));
  expect(latest!.showInitialLoader).toBe(false);
});

it("does not keep cached messages that were deleted while away", async () => {
  const { rerender } = render(<Harness convId="a" />);
  await answer("a", [msg("a1", 1), msg("a2", 2)]);
  rerender(<Harness convId="b" />);
  await answer("b", [msg("b1", 1)]);
  rerender(<Harness convId="a" />);
  expect(latest!.messages.map((m) => m.id)).toEqual(["a1", "a2"]);
  await answer("a", [msg("a1", 1)]);
  await waitFor(() => expect(latest!.messages.map((m) => m.id)).toEqual(["a1"]));
});

it.each(["CHANNEL_NOT_FOUND", "COMMUNITY_NOT_MEMBER"])(
  "drops a cached chat when the quiet refresh fails with %s",
  async (code) => {
    const { rerender } = render(<Harness convId="a" />);
    await answer("a", [msg("a1", 1), msg("a2", 2)]);
    rerender(<Harness convId="b" />);
    await answer("b", [msg("b1", 1)]);
    rerender(<Harness convId="a" />);
    expect(latest!.messages.map((m) => m.id)).toEqual(["a1", "a2"]);
    await act(async () => {
      failing.a?.(new ApiError("gone", 404, code));
      await Promise.resolve();
    });
    await waitFor(() => expect(latest!.messages).toEqual([]));
    expect(latest!.accessError).toBe("gone");
    expect(hasCachedConversation("a")).toBe(false);
  },
);

it("waits for the quiet refresh before loading older messages", async () => {
  const page = Array.from({ length: 50 }, (_, i) => ({ ...msg(`p${i}`, 0), createdAt: new Date(Date.UTC(2026, 9, 1, 0, i)).toISOString() }));
  const { rerender } = render(<Harness convId="a" />);
  await answer("a", page);
  rerender(<Harness convId="b" />);
  await answer("b", [msg("b1", 1)]);
  olderCalls.length = 0;
  rerender(<Harness convId="a" />);
  await act(async () => {
    await latest!.handleScroll();
  });
  expect(olderCalls).toEqual([]);
});

it("does not keep the loader of an abandoned chat over a cached one", async () => {
  const { rerender } = render(<Harness convId="a" />);
  await answer("a", [msg("a1", 1)]);
  rerender(<Harness convId="b" />);
  expect(latest!.showInitialLoader).toBe(true);
  rerender(<Harness convId="a" />);
  expect(latest!.showInitialLoader).toBe(false);
  expect(latest!.messages.map((m) => m.id)).toEqual(["a1"]);
});

it("waits for the quiet refresh before syncing history after a reconnect", async () => {
  const { rerender } = render(<Harness convId="a" />);
  await answer("a", [msg("a1", 1)]);
  rerender(<Harness convId="b" />);
  await answer("b", [msg("b1", 1)]);
  callCount.a = 0;
  rerender(<Harness convId="a" tick={1} />);
  expect(callCount.a).toBe(1);
  await answer("a", [msg("a1", 1), msg("a2", 2)]);
  await waitFor(() => expect(callCount.a).toBe(2));
});

it("jumps to a message only after a cached chat has been reconciled", async () => {
  const { rerender } = render(<Harness convId="a" />);
  await answer("a", [msg("a1", 1)]);
  rerender(<Harness convId="b" />);
  await answer("b", [msg("b1", 1)]);
  callCount.a = 0;
  rerender(<Harness convId="a" />);
  expect(callCount.a).toBe(1);
  let jumped = false;
  act(() => {
    void latest!.jumpToMessage(msg("old", 0)).then(() => (jumped = true));
  });
  await act(async () => {
    await Promise.resolve();
  });
  expect(callCount.a).toBe(1);
  await answer("a", [msg("a1", 1), msg("a2", 2)]);
  await waitFor(() => expect(callCount.a).toBeGreaterThan(1));
  expect(jumped).toBe(false);
});

it("lets older pages load after a refresh that stops before the start of history", async () => {
  render(<Harness convId="r" />);
  await answer("r", []);
  const full = Array.from({ length: PAGE_LIMIT }, (_, i) => msg(`r${i}`, i % 60));
  let refreshing: Promise<void> = Promise.resolve();
  act(() => {
    refreshing = latest!.refreshHistory();
  });
  await answer("r", full);
  await act(async () => {
    await refreshing;
  });
  olderCalls.length = 0;
  act(() => {
    latest!.handleScroll();
  });
  expect(olderCalls.some((c) => c.startsWith("r:"))).toBe(true);
});
