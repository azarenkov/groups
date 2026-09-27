import type { ChannelMessage } from "../../../../shared/types/channel";
import { sharesTimeWithNext } from "../messageTime";

const msg = (over: Partial<ChannelMessage>): ChannelMessage => ({
  id: Math.random().toString(),
  channelId: "c",
  authorId: "a",
  content: "hi",
  createdAt: "2026-09-25T18:41:05.000Z",
  editedAt: null,
  ...over,
});

describe("sharesTimeWithNext", () => {
  it("hides the time of a message followed by one from the same author in the same minute", () => {
    expect(sharesTimeWithNext(msg({}), msg({ createdAt: "2026-09-25T18:41:59.000Z" }))).toBe(true);
  });

  it("keeps the time when the minute, the author or the kind differs", () => {
    expect(sharesTimeWithNext(msg({}), msg({ createdAt: "2026-09-25T18:42:00.000Z" }))).toBe(false);
    expect(sharesTimeWithNext(msg({}), msg({ authorId: "b" }))).toBe(false);
    expect(sharesTimeWithNext(msg({}), msg({ callStatus: "completed" }))).toBe(false);
    expect(sharesTimeWithNext(msg({}), undefined)).toBe(false);
  });
});
