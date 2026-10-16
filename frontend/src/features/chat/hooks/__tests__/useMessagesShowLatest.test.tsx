import React from "react";
import { act, render, waitFor } from "@testing-library/react";

import type { ChannelMessage } from "../../../../shared/types/channel";
import { clearMessageCache } from "../../lib/messageCache";
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
  getMessage: async () => target,
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

beforeEach(() => clearMessageCache());

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
