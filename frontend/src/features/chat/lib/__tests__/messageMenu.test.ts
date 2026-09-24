import { messageActions, messageCopyText } from "../messageMenu";
import type { ChannelMessage } from "../../../../shared/types/channel";

const msg = (over: Partial<ChannelMessage> = {}): ChannelMessage => ({
  id: "m",
  channelId: "c",
  authorId: "a",
  content: "hello",
  createdAt: "2026-09-24T10:00:00Z",
  editedAt: null,
  ...over,
});

const ctx = (over: Partial<Parameters<typeof messageActions>[0]> = {}) => ({
  msg: msg(),
  isMine: false,
  canModerate: false,
  hasSelection: false,
  ...over,
});

describe("messageActions", () => {
  it("lets you copy, edit and delete your own message", () => {
    expect(messageActions(ctx({ isMine: true }))).toEqual(["reply", "copy", "link", "edit", "delete"]);
  });

  it("lets you copy and report someone else's message", () => {
    expect(messageActions(ctx())).toEqual(["reply", "copy", "link", "report"]);
  });

  it("adds delete for moderators", () => {
    expect(messageActions(ctx({ canModerate: true }))).toEqual(["reply", "copy", "link", "delete", "report"]);
  });

  it("offers only delete on your own call card", () => {
    expect(messageActions(ctx({ isMine: true, msg: msg({ callStatus: "completed", content: "" }) }))).toEqual([
      "link",
      "delete",
    ]);
  });

  it("offers copy of a selection even on an empty message", () => {
    expect(messageActions(ctx({ hasSelection: true, msg: msg({ content: "" }) }))).toEqual(["reply", "copy", "link", "report"]);
  });
});

describe("messageCopyText", () => {
  it("copies the selection when there is one", () => {
    expect(messageCopyText(msg(), "ell")).toBe("ell");
  });

  it("copies the whole message with attachment names otherwise", () => {
    const withFiles = msg({
      attachments: [{ id: "f", fileName: "report.pdf" } as never],
    });
    expect(messageCopyText(withFiles, "")).toBe("hello\nreport.pdf");
  });
});
