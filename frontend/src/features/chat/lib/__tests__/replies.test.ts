import {
  REPLY_SNIPPET_CHARS,
  markReplyDeleted,
  refreshReplySnippet,
  replySnippet,
} from "../replies";
import type { ChannelMessage } from "../../../../shared/types/channel";

const msg = (id: string, replyTo?: ChannelMessage["replyTo"]): ChannelMessage => ({
  id,
  channelId: "c",
  authorId: "a",
  content: id,
  createdAt: "2026-09-24T10:00:00Z",
  editedAt: null,
  replyTo,
});

const quote = (id: string) => ({ id, authorId: "b", authorName: "Bob", content: "old", hasAttachments: false, deleted: false });

describe("replySnippet", () => {
  it("flattens line breaks like the server", () => {
    expect(replySnippet(" one\n\ntwo   three ")).toBe("one two three");
  });

  it("cuts long text with an ellipsis", () => {
    const cut = replySnippet("я".repeat(REPLY_SNIPPET_CHARS + 10));
    expect(Array.from(cut)).toHaveLength(REPLY_SNIPPET_CHARS + 1);
    expect(cut.endsWith("…")).toBe(true);
  });
});

describe("live quote updates", () => {
  it("marks quotes of a deleted message as deleted", () => {
    const out = markReplyDeleted([msg("r", quote("x")), msg("s")], "x");
    expect(out[0].replyTo).toEqual({ id: "x", content: "", hasAttachments: false, deleted: true });
    expect(out[1].replyTo).toBeUndefined();
  });

  it("refreshes the quoted text after an edit", () => {
    const out = refreshReplySnippet([msg("r", quote("x"))], "x", "new\ntext");
    expect(out[0].replyTo?.content).toBe("new text");
    expect(out[0].replyTo?.authorName).toBe("Bob");
  });

  it("refreshes the attachment flag of a quote after an edit", () => {
    const withFile = refreshReplySnippet([msg("r", quote("x"))], "x", "", true);
    expect(withFile[0].replyTo?.hasAttachments).toBe(true);
    const kept = refreshReplySnippet(withFile, "x", "text");
    expect(kept[0].replyTo?.hasAttachments).toBe(true);
    const removed = refreshReplySnippet(withFile, "x", "text", false);
    expect(removed[0].replyTo?.hasAttachments).toBe(false);
  });

  it("leaves the list untouched when nothing quotes the message", () => {
    const list = [msg("r")];
    expect(markReplyDeleted(list, "x")).toBe(list);
    expect(refreshReplySnippet(list, "x", "t")).toBe(list);
  });
});
