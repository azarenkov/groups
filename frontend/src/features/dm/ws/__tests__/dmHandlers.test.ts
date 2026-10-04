import { dmHandlers } from "../dmHandlers";
import { chatHandlers } from "../../../chat/ws/chatHandlers";
import { useAuthStore } from "../../../auth/store/authStore";
import { useUnreadStore } from "../../../chat/store/unreadStore";
import { useNotificationPrefsStore } from "../../../settings/store/notificationPrefsStore";
import { playNotificationSound } from "../../../../shared/lib/notificationSound";
import { showDesktopNotification } from "../../../../shared/lib/desktopNotification";

jest.mock("../../../../shared/lib/notificationSound", () => ({ playNotificationSound: jest.fn() }));
jest.mock("../../../../shared/lib/desktopNotification", () => ({ showDesktopNotification: jest.fn() }));
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
