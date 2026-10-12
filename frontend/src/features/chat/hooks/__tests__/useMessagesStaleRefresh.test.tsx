import React from "react";
import { act, render, waitFor } from "@testing-library/react";

import type { ChannelMessage } from "../../../../shared/types/channel";
import { clearMessageCache } from "../../lib/messageCache";
import type { Messenger } from "../useMessenger";
import { useMessages } from "../useMessages";
import { ws, type WsEnvelope } from "../../../../shared/ws/websocket";

const PARTITION = "messaging:stale";

const msg = (id: string, content: string): ChannelMessage => ({
  id,
  channelId: "c",
  authorId: "u2",
  content,
  createdAt: "2026-10-01T10:01:00Z",
  editedAt: null,
});

const replies: Array<(messages: ChannelMessage[], seq: number) => void> = [];
const messenger = {
  listMessages: () =>
    new Promise((resolve) => {
      replies.push((messages, seq) => resolve({ messages, partition: PARTITION, seq }));
    }),
} as unknown as Messenger;

let latest: ReturnType<typeof useMessages> | null = null;
const Harness: React.FC = () => {
  latest = useMessages({
    messenger,
    convId: "stale",
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
  ws.disconnect();
});

it("reloads a refreshed history that is older than events already applied", async () => {
  render(<Harness />);
  await act(async () => {
    replies.shift()!([msg("m1", "first")], 1);
  });
  await waitFor(() => expect(latest!.messages.map((m) => m.content)).toEqual(["first"]));

  let refreshing: Promise<void> | undefined;
  act(() => {
    refreshing = latest!.refreshHistory();
  });
  ws.receive({ type: "message.edited", data: {}, partition: PARTITION, seq: 2 } as unknown as WsEnvelope);
  await act(async () => {
    replies.shift()!([msg("m1", "first")], 1);
  });
  await waitFor(() => expect(replies.length).toBe(1));
  await act(async () => {
    replies.shift()!([msg("m1", "edited")], 2);
    await refreshing;
  });
  expect(latest!.messages.map((m) => m.content)).toEqual(["edited"]);
});

it("does not apply a refreshed history that stays older than live events", async () => {
  render(<Harness />);
  await act(async () => {
    replies.shift()!([msg("m1", "first")], 1);
  });
  await waitFor(() => expect(latest!.messages.map((m) => m.content)).toEqual(["first"]));
  ws.receive({ type: "message.edited", data: {}, partition: PARTITION, seq: 2 } as unknown as WsEnvelope);

  jest.useFakeTimers();
  try {
    let refreshing: Promise<void> | undefined;
    act(() => {
      refreshing = latest!.refreshHistory();
    });
    const flush = async () => {
      for (let i = 0; i < 10; i++) await Promise.resolve();
    };
    for (let attempt = 0; attempt < 5; attempt++) {
      await act(async () => {
        replies.shift()?.([], 1);
        await flush();
        jest.advanceTimersByTime(5000);
        await flush();
      });
    }
    await act(async () => {
      await refreshing;
    });
    expect(replies).toHaveLength(0);
    expect(latest!.messages.map((m) => m.content)).toEqual(["first"]);
  } finally {
    jest.useRealTimers();
  }
});
