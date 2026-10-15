import { dmHandlers } from "../dmHandlers";
import { chatHandlers } from "../../../chat/ws/chatHandlers";
import { useAuthStore } from "../../../auth/store/authStore";
import { refetchUnread, useUnreadStore } from "../../../chat/store/unreadStore";
import { useNotificationPrefsStore } from "../../../settings/store/notificationPrefsStore";
import { playNotificationSound } from "../../../../shared/lib/notificationSound";
import { showDesktopNotification } from "../../../../shared/lib/desktopNotification";
import { useHiddenDmStore } from "../../store/hiddenDmStore";
import { queryClient, queryKeys } from "../../../../shared/query/queryClient";
import { refetchDmPreviews, useDmPreviewStore } from "../../store/dmPreviewStore";

jest.mock("../../../../shared/lib/notificationSound", () => ({ playNotificationSound: jest.fn() }));
jest.mock("../../../../shared/lib/desktopNotification", () => ({ showDesktopNotification: jest.fn() }));
jest.mock("../../store/dmPreviewStore", () => {
  const actual = jest.requireActual("../../store/dmPreviewStore");
  return { ...actual, refetchDmPreviews: jest.fn() };
});
jest.mock("../../../chat/store/unreadStore", () => {
  const actual = jest.requireActual("../../../chat/store/unreadStore");
  return { ...actual, refetchUnread: jest.fn() };
});
jest.mock("../../../settings/store/notificationPrefsStore", () => {
  const actual = jest.requireActual("../../../settings/store/notificationPrefsStore");
  return { ...actual, notificationPrefsLoaded: () => true };
});
jest.mock("../../../settings/store/notificationSettingsStore", () => {
  const actual = jest.requireActual("../../../settings/store/notificationSettingsStore");
  return { ...actual, defaultMentionLevel: () => "all", messageSoundAllowed: () => true, desktopNotifAllowed: () => true };
});
jest.mock("../../../social/store/blockStore", () => {
  const actual = jest.requireActual("../../../social/store/blockStore");
  actual.useBlockStore.setState({ loaded: true, blocked: {} });
  return { ...actual, isBlockedEitherWay: () => false };
});

const sound = playNotificationSound as jest.Mock;
const desktop = showDesktopNotification as jest.Mock;

beforeEach(() => {
  sound.mockReset();
  desktop.mockReset();
  useAuthStore.setState({ user: { id: "me", name: "Me" } } as never);
  useUnreadStore.setState({ byDm: {}, byChannel: {} } as never);
  useHiddenDmStore.setState({ hidden: [], loaded: true });
  (refetchDmPreviews as jest.Mock).mockReset();
  (refetchUnread as jest.Mock).mockReset().mockResolvedValue(undefined);
});

it("keeps counting a muted DM but stays silent", () => {
  useNotificationPrefsStore.setState({ dmLevels: { peer: "none" } });
  dmHandlers["dm_message.sent"]!({
    id: "m1",
    senderId: "peer",
    recipientId: "me",
    senderName: "Peer",
    content: "hi",
    occurredAt: new Date().toISOString(),
  });
  expect(useUnreadStore.getState().byDm.peer).toBe(1);
  expect(sound).not.toHaveBeenCalled();
  expect(desktop).not.toHaveBeenCalled();
});

it("still notifies for an unmuted DM", () => {
  useNotificationPrefsStore.setState({ dmLevels: {} });
  dmHandlers["dm_message.sent"]!({
    id: "m2",
    senderId: "peer",
    recipientId: "me",
    senderName: "Peer",
    content: "hi",
    occurredAt: new Date().toISOString(),
  });
  expect(useUnreadStore.getState().byDm.peer).toBe(1);
  expect(sound).toHaveBeenCalledTimes(1);
  expect(desktop).toHaveBeenCalledTimes(1);
});

it("keeps counting a muted channel but stays silent", () => {
  useNotificationPrefsStore.setState({ channelLevels: { ch: "none" } });
  chatHandlers["message.sent"]!({
    id: "m3",
    channelId: "ch",
    authorId: "peer",
    content: "hi",
    occurredAt: new Date().toISOString(),
  });
  expect(useUnreadStore.getState().byChannel.ch).toBe(1);
  expect(sound).not.toHaveBeenCalled();
  expect(desktop).not.toHaveBeenCalled();
});

it("keeps counting a hidden DM but stays silent", () => {
  useNotificationPrefsStore.setState({ dmLevels: {} });
  useHiddenDmStore.setState({ hidden: ["peer"] });
  dmHandlers["dm_message.sent"]!({
    id: "m4",
    senderId: "peer",
    recipientId: "me",
    senderName: "Peer",
    content: "hi",
    occurredAt: new Date().toISOString(),
  });
  expect(useUnreadStore.getState().byDm.peer).toBe(1);
  expect(useHiddenDmStore.getState().hidden).toEqual(["peer"]);
  expect(sound).not.toHaveBeenCalled();
  expect(desktop).not.toHaveBeenCalled();
});

it("rereads hidden chats from the server when they change elsewhere", () => {
  const changed = jest.spyOn(useHiddenDmStore.getState(), "remoteChanged").mockImplementation(() => {});
  dmHandlers["dm_conversation.hidden_changed"]!({ peerId: "peer", hidden: true });
  dmHandlers["dm_conversation.hidden_changed"]!({});
  expect(changed).toHaveBeenCalledTimes(1);
  expect(changed).toHaveBeenCalledWith("peer", true);
  changed.mockRestore();
});

it("reloads previews when a conversation is cleared", () => {
  dmHandlers["dm_conversation.cleared"]!({
    participants: ["peer", "me"],
    scope: "everyone",
    clearedAt: new Date().toISOString(),
  });
  expect(refetchDmPreviews).toHaveBeenCalledWith(true);
  (refetchDmPreviews as jest.Mock).mockReset();
  dmHandlers["dm_conversation.cleared"]!({
    participants: ["a", "b"],
    scope: "everyone",
    clearedAt: new Date().toISOString(),
  });
  expect(refetchDmPreviews).not.toHaveBeenCalled();
});

it("stays silent until hidden chats are known", () => {
  useNotificationPrefsStore.setState({ dmLevels: {} });
  useHiddenDmStore.setState({ hidden: [], loaded: false });
  dmHandlers["dm_message.sent"]!({
    id: "m5",
    senderId: "peer",
    recipientId: "me",
    senderName: "Peer",
    content: "hi",
    occurredAt: new Date().toISOString(),
  });
  expect(useUnreadStore.getState().byDm.peer).toBe(1);
  expect(sound).not.toHaveBeenCalled();
  expect(desktop).not.toHaveBeenCalled();
});

it("drops global search results when a conversation is cleared", () => {
  const invalidate = jest.spyOn(queryClient, "invalidateQueries");
  dmHandlers["dm_conversation.cleared"]!({
    participants: ["peer", "me"],
    scope: "me",
    clearedAt: new Date().toISOString(),
  });
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ["globalSearch"] });
  invalidate.mockRestore();
});

it("removes the cleared conversation preview right away", () => {
  useDmPreviewStore.getState().push("peer", {
    messageId: "m9",
    senderId: "peer",
    content: "secret",
    callStatus: null,
    hasAttachments: false,
    createdAt: "2026-10-11T09:00:00Z",
  });
  dmHandlers["dm_conversation.cleared"]!({
    participants: ["peer", "me"],
    scope: "everyone",
    clearedAt: "2026-10-11T10:00:00Z",
  });
  expect(useDmPreviewStore.getState().byFriend.peer).toBeUndefined();
});

it("drops cleared pins from the cache before refetching them", () => {
  const pin = (messageId: string, createdAt: string) => ({
    messageId,
    authorId: "peer",
    authorName: "Peer",
    content: messageId,
    hasAttachments: false,
    createdAt,
    pinnedBy: "peer",
    pinnedAt: createdAt,
  });
  queryClient.setQueryData(queryKeys.pins("dm:peer"), [
    pin("old", "2026-10-11T09:00:00Z"),
    pin("new", "2026-10-11T11:00:00Z"),
  ]);
  dmHandlers["dm_conversation.cleared"]!({
    participants: ["peer", "me"],
    scope: "me",
    clearedAt: "2026-10-11T10:30:00Z",
  });
  expect(
    (queryClient.getQueryData(queryKeys.pins("dm:peer")) as { messageId: string }[]).map((p) => p.messageId),
  ).toEqual(["new"]);
  queryClient.removeQueries({ queryKey: queryKeys.pins("dm:peer") });
});

it("clears the unread badge of a cleared chat right away unless a newer message came in", () => {
  useUnreadStore.setState({ byDm: { peer: 3 } });
  dmHandlers["dm_conversation.cleared"]!({
    participants: ["peer", "me"],
    scope: "me",
    clearedAt: "2026-10-11T12:00:00Z",
  });
  expect(useUnreadStore.getState().byDm.peer).toBeUndefined();
});

it("drops cleared DM hits from cached global search", () => {
  const hit = (messageId: string, peerId: string, createdAt: string) => ({
    messageId,
    authorId: peerId,
    authorName: peerId,
    createdAt,
    place: { type: "dm" as const, peerId, peerName: peerId },
    snippet: messageId,
  });
  const key = queryKeys.globalSearch("messages", "hi");
  queryClient.setQueryData(key, {
    pages: [
      {
        items: [
          hit("old", "peer", "2026-10-11T09:00:00Z"),
          hit("new", "peer", "2026-10-11T11:00:00Z"),
          hit("other", "friend", "2026-10-11T09:00:00Z"),
        ],
        nextCursor: null,
      },
    ],
    pageParams: [null],
  });
  dmHandlers["dm_conversation.cleared"]!({
    participants: ["peer", "me"],
    scope: "everyone",
    clearedAt: "2026-10-11T10:00:00Z",
  });
  const data = queryClient.getQueryData(key) as { pages: { items: { messageId: string }[] }[] };
  expect(data.pages[0].items.map((h) => h.messageId)).toEqual(["new", "other"]);
  queryClient.removeQueries({ queryKey: key });
});

it("rereads unread counts when an unread message to me is deleted", async () => {
  useUnreadStore.setState({ byDm: { peer: 1 } } as never);
  dmHandlers["dm_message.deleted"]!({ id: "m1", senderId: "peer", recipientId: "me" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(refetchUnread).toHaveBeenCalledTimes(1);
  (refetchUnread as jest.Mock).mockClear();
  dmHandlers["dm_message.deleted"]!({ id: "m2", senderId: "me", recipientId: "peer" });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(refetchUnread).not.toHaveBeenCalled();
});

it("raises a missed call like an unread message with a call notice", async () => {
  const { useCallStore } = await import("../../../voice/store/callStore");
  useCallStore.setState({ callNotice: null });
  dmHandlers["dm_message.sent"]!({
    id: "call-1",
    senderId: "peer",
    senderName: "Peer",
    recipientId: "me",
    content: "call",
    callStatus: "missed",
    occurredAt: new Date().toISOString(),
  });
  expect(useUnreadStore.getState().byDm.peer).toBe(1);
  expect(useCallStore.getState().callNotice).toEqual({ kind: "missed", userId: "peer", name: "Peer" });
  expect(desktop).toHaveBeenCalledWith("Peer", "Missed call");
});

it("keeps answered and declined call cards out of unread and notifications", () => {
  for (const callStatus of ["completed", "declined"]) {
    dmHandlers["dm_message.sent"]!({
      id: `call-${callStatus}`,
      senderId: "peer",
      senderName: "Peer",
      recipientId: "me",
      content: "call",
      callStatus,
      occurredAt: new Date().toISOString(),
    });
  }
  expect(useUnreadStore.getState().byDm.peer).toBeUndefined();
  expect(sound).not.toHaveBeenCalled();
  expect(desktop).not.toHaveBeenCalled();
});

it("does not raise a missed call that was already read before its card arrived", () => {
  const endedAt = new Date(Date.now() - 60_000).toISOString();
  dmHandlers["dm.read"]!({ readerId: "me", peerId: "late", readAt: new Date(Date.now() - 30_000).toISOString() });
  dmHandlers["dm_message.sent"]!({
    id: "call-late",
    senderId: "late",
    senderName: "Late",
    recipientId: "me",
    content: "call",
    callStatus: "missed",
    occurredAt: endedAt,
  });
  expect(useUnreadStore.getState().byDm.late).toBeUndefined();
  expect(desktop).not.toHaveBeenCalled();
});

it("forgets the previous account's read marks when the session resets", async () => {
  const { forgetDmTracking } = await import("../../../../shared/ws/context");
  dmHandlers["dm.read"]!({ readerId: "me", peerId: "shared", readAt: new Date().toISOString() });
  forgetDmTracking();
  dmHandlers["dm_message.sent"]!({
    id: "call-shared",
    senderId: "shared",
    senderName: "Shared",
    recipientId: "me",
    content: "call",
    callStatus: "missed",
    occurredAt: new Date(Date.now() - 60_000).toISOString(),
  });
  expect(useUnreadStore.getState().byDm.shared).toBe(1);
});
