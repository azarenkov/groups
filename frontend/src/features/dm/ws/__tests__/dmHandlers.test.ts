import { dmHandlers } from "../dmHandlers";
import { chatHandlers } from "../../../chat/ws/chatHandlers";
import { useAuthStore } from "../../../auth/store/authStore";
import { useUnreadStore } from "../../../chat/store/unreadStore";
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
    inviteCommunityId: null,
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
