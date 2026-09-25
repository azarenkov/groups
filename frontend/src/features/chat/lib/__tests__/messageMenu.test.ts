import { canDeleteForEveryone, messageActions, messageCopyText } from "../messageMenu";
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
  hasSelection: false,
  ...over,
});

describe("messageActions", () => {
  it("lets you copy, edit and delete your own message", () => {
    expect(messageActions(ctx({ isMine: true }))).toEqual(["reply", "forward", "copy", "link", "edit", "delete"]);
  });

  it("lets you copy, hide and report someone else's message", () => {
    expect(messageActions(ctx())).toEqual(["reply", "forward", "copy", "link", "delete", "report"]);
  });

  it("offers only delete on your own call card", () => {
    expect(messageActions(ctx({ isMine: true, msg: msg({ callStatus: "completed", content: "" }) }))).toEqual([
      "link",
      "delete",
    ]);
  });

  it("does not let the recipient hide an invite they have not answered", () => {
    expect(messageActions(ctx({ msg: msg({ inviteCommunityId: "s", content: "" }) }))).toEqual(["link"]);
  });

  it("does not offer edit on a forwarded message", () => {
    const forwarded = msg({
      forwardedFrom: {
        messageId: "o",
        authorId: "b",
        authorName: "Bob",
        sentAt: "2026-09-24T09:00:00Z",
        source: { kind: "dm" },
      },
    });
    expect(messageActions(ctx({ isMine: true, msg: forwarded }))).not.toContain("edit");
  });

  it("offers copy of a selection even on an empty message", () => {
    expect(messageActions(ctx({ hasSelection: true, msg: msg({ content: "" }) }))).toEqual(["reply", "forward", "copy", "link", "delete", "report"]);
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

describe("canDeleteForEveryone", () => {
  it("allows your own message", () => {
    expect(canDeleteForEveryone(msg(), true, false)).toBe(true);
  });

  it("only hides someone else's message for a regular member", () => {
    expect(canDeleteForEveryone(msg(), false, false)).toBe(false);
  });

  it("allows moderators to delete anyone's message", () => {
    expect(canDeleteForEveryone(msg(), false, true)).toBe(true);
  });

  it("keeps your own invite card deletable only for you", () => {
    expect(canDeleteForEveryone(msg({ inviteCommunityId: "s" }), true, false)).toBe(false);
  });
});
