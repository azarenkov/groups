import type { ChannelMessage } from "../../../../shared/types/channel";
import {
  MESSAGE_CACHE_LIMIT,
  clearMessageCache,
  hasCachedConversation,
  installMessageCacheSync,
  readConversation,
  reconcileLatestPage,
  saveConversation,
  trimCachedConversations,
} from "../messageCache";
import { useAuthStore } from "../../../auth/store/authStore";

const listeners: ((env: { type: string; data: unknown }) => void)[] = [];
const openListeners: (() => void)[] = [];
jest.mock("../../../../shared/ws/websocket", () => ({
  ws: {
    on: (fn: (env: { type: string; data: unknown }) => void) => {
      listeners.push(fn);
      return () => undefined;
    },
    onOpen: (fn: () => void) => {
      openListeners.push(fn);
      return () => undefined;
    },
  },
}));

const msg = (id: string, minute: number, extra: Partial<ChannelMessage> = {}): ChannelMessage => ({
  id,
  channelId: "c1",
  authorId: "u2",
  content: id,
  createdAt: `2026-10-01T10:${String(minute).padStart(2, "0")}:00Z`,
  editedAt: null,
  ...extra,
});

const entry = (messages: ChannelMessage[]) => ({
  messages,
  hasMore: false,
  scroll: { atBottom: true, anchorId: null, offset: 0 },
});

const emit = (type: string, data: unknown) => listeners.forEach((fn) => fn({ type, data }));

beforeAll(() => {
  installMessageCacheSync();
});

beforeEach(() => {
  clearMessageCache();
  useAuthStore.setState({ user: { id: "me", name: "Me" } } as never);
});

describe("reconcileLatestPage", () => {
  it("replaces the whole history when the latest page is short", () => {
    const result = reconcileLatestPage([msg("a", 1), msg("gone", 2)], [msg("a", 1), msg("b", 3)], false, new Set(["a", "gone"]));
    expect(result.messages.map((m) => m.id)).toEqual(["a", "b"]);
  });

  it("keeps older cached messages and replaces the refreshed window", () => {
    const cached = [msg("old", 1), msg("x", 5), msg("deleted", 6)];
    const latest = [msg("x", 5), msg("y", 7)];
    const result = reconcileLatestPage(cached, latest, true, new Set(cached.map((m) => m.id)));
    expect(result).toEqual({ messages: [msg("old", 1), msg("x", 5), msg("y", 7)], gap: false });
  });

  it("drops the cached history when it no longer touches the latest page", () => {
    const result = reconcileLatestPage([msg("old", 1)], [msg("y", 7), msg("z", 8)], true, new Set(["old"]));
    expect(result).toEqual({ messages: [msg("y", 7), msg("z", 8)], gap: true });
  });

  it("drops cached messages the server no longer has, even newer ones", () => {
    const result = reconcileLatestPage([msg("a", 1), msg("deleted-later", 9)], [], false, new Set(["a", "deleted-later"]));
    expect(result.messages).toEqual([]);
    const partial = reconcileLatestPage([msg("a", 1), msg("x", 9)], [msg("a", 1)], false, new Set(["a", "x"]));
    expect(partial.messages.map((m) => m.id)).toEqual(["a"]);
  });

  it("does not treat a live message as an overlap with the refreshed page", () => {
    const current = [msg("old", 1), msg("live", 9)];
    const result = reconcileLatestPage(current, [msg("y", 7), msg("z", 8)], true, new Set(["old"]));
    expect(result).toEqual({ messages: [msg("y", 7), msg("z", 8), msg("live", 9)], gap: true });
  });

  it("keeps live messages that arrived after the page was read", () => {
    const result = reconcileLatestPage([msg("a", 1), msg("live", 9)], [msg("a", 1)], false, new Set(["a"]));
    expect(result.messages.map((m) => m.id)).toEqual(["a", "live"]);
  });
});

describe("message cache", () => {
  it("evicts the least recently opened conversations", () => {
    for (let i = 0; i <= MESSAGE_CACHE_LIMIT; i += 1) saveConversation(`c${i}`, entry([msg("a", 1)]));
    expect(hasCachedConversation("c0")).toBe(false);
    readConversation("c1");
    saveConversation("extra", entry([]));
    expect(hasCachedConversation("c1")).toBe(true);
    expect(hasCachedConversation("c2")).toBe(false);
  });

  it("applies deletes, edits and hides to closed conversations", () => {
    saveConversation("c1", entry([msg("a", 1), msg("b", 2, { replyTo: { id: "a", authorId: "u2", content: "a", deleted: false } as never })]));
    saveConversation("u2", entry([msg("d1", 1), msg("d2", 2)]));
    emit("message.edited", { id: "b", channelId: "c1", newContent: "B", occurredAt: "2026-10-01T11:00:00Z" });
    emit("message.deleted", { id: "a", channelId: "c1" });
    emit("dm_message.deleted", { id: "d1", senderId: "u2", recipientId: "me" });
    emit("message.hidden", { messageId: "d2" });
    const channel = readConversation("c1")!.messages;
    expect(channel.map((m) => m.id)).toEqual(["b"]);
    expect(channel[0].content).toBe("B");
    expect(channel[0].replyTo?.deleted).toBe(true);
    expect(readConversation("u2")!.messages).toEqual([]);
  });

  it("finds the cached DM of an edit that only names the sender", () => {
    saveConversation("me-peer", entry([msg("mine", 1, { authorId: "me" })]));
    emit("dm_message.edited", { id: "mine", senderId: "me", newContent: "edited", occurredAt: "2026-10-01T11:00:00Z" });
    expect(readConversation("me-peer")!.messages[0].content).toBe("edited");
  });

  it("keeps only the newest page of every chat after reconnecting", () => {
    const long = Array.from({ length: 120 }, (_, i) => msg(`m${i}`, i % 60, { createdAt: new Date(Date.UTC(2026, 9, 1, 0, i)).toISOString() }));
    saveConversation("c1", { ...entry(long), scroll: { atBottom: false, anchorId: "m3", offset: 10 } });
    openListeners.forEach((fn) => fn());
    const trimmed = readConversation("c1")!;
    expect(trimmed.messages).toHaveLength(50);
    expect(trimmed.hasMore).toBe(true);
    expect(trimmed.scroll.atBottom).toBe(true);
    trimCachedConversations();
  });
});
