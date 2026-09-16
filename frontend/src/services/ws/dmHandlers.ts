import type { WsHandlers } from "./types";
import { useAuthStore } from "../../store/authStore";
import { isBlockedEitherWay } from "../../store/blockStore";
import { refetchDmPreviews, useDmPreviewStore } from "../../store/dmPreviewStore";
import { useFriendshipStore } from "../../store/friendshipStore";
import { notificationPrefsLoaded, useNotificationPrefsStore } from "../../store/notificationPrefsStore";
import { defaultMentionLevel, desktopNotifAllowed, messageSoundAllowed } from "../../store/notificationSettingsStore";
import { useReadReceiptStore } from "../../store/readReceiptStore";
import { useUnreadStore } from "../../store/unreadStore";
import { useUserNamesStore } from "../../store/userNamesStore";
import { showDesktopNotification } from "../../utils/desktopNotification";
import { playNotificationSound } from "../../utils/notificationSound";
import { asString, eventTs, getActiveDmView, lastDmBumpAt, scheduleUnreadRefetch, throttleMarkDmRead, viewIsForeground } from "./context";

export const dmHandlers: WsHandlers = {
  "dm_message.sent": (d) => {
      const senderId = asString(d.senderId);
      const recipientId = asString(d.recipientId);
      const selfId = useAuthStore.getState().user?.id;
      if (!senderId || !recipientId || !selfId) return;
      const peerId = senderId === selfId ? recipientId : senderId;
      if (isBlockedEitherWay(peerId)) return;
      const peerName =
        (senderId !== selfId ? asString(d.senderName) : undefined) ??
        useUserNamesStore.getState().overrides[peerId] ??
        useFriendshipStore.getState().friends.find((f) => f.friendId === peerId)?.friendName;
      const knownPeerName = peerName ?? useDmPreviewStore.getState().byFriend[peerId]?.peerName;
      useDmPreviewStore.getState().push(peerId, {
        messageId: asString(d.id) ?? "",
        peerName,
        senderId,
        content: asString(d.content) ?? "",
        callStatus: asString(d.callStatus) ?? null,
        inviteServerId: asString(d.inviteServerId) ?? null,
        hasAttachments: Array.isArray(d.attachments) && d.attachments.length > 0,
        createdAt: asString(d.occurredAt) ?? new Date().toISOString(),
      });
      if (!knownPeerName) refetchDmPreviews();
      if (recipientId !== selfId) return;
      if (getActiveDmView() === senderId && viewIsForeground()) {
        throttleMarkDmRead(senderId);
        return;
      }
      if (!notificationPrefsLoaded()) {
        scheduleUnreadRefetch();
        return;
      }
      const dmLevel =
        useNotificationPrefsStore.getState().dmLevels[senderId] ??
        (defaultMentionLevel() === "none" ? "none" : "all");
      if (dmLevel === "none") return;
      lastDmBumpAt[senderId] = Math.max(
        lastDmBumpAt[senderId] ?? 0,
        eventTs(d.occurredAt),
      );
      useUnreadStore.getState().bumpDm(senderId);
      if (messageSoundAllowed()) playNotificationSound();
      if (desktopNotifAllowed()) {
        showDesktopNotification(peerName ?? "Rift", asString(d.content) ?? "");
      }
      return;
  },
  "dm_message.edited": (d) => {
      const id = asString(d.id);
      const newContent = asString(d.newContent);
      if (id && newContent != null) {
        useDmPreviewStore
          .getState()
          .editMessage(
            id,
            newContent,
            Array.isArray(d.attachments)
              ? d.attachments.length > 0
              : undefined,
          );
      }
      return;
  },
  "dm_message.deleted": (d) => {
      const id = asString(d.id);
      if (id && useDmPreviewStore.getState().hasMessage(id)) {
        refetchDmPreviews(true);
      }
      return;
  },
  "dm.read": (d) => {
      const readerId = asString(d.readerId);
      const readAt = asString(d.readAt);
      const selfId = useAuthStore.getState().user?.id;
      if (readerId && readerId === selfId) {
        const peerId = asString(d.peerId);
        if (!peerId) return;
        const readTs = Date.parse(readAt ?? "");
        const bumpTs = lastDmBumpAt[peerId];
        if (
          bumpTs !== undefined &&
          (!Number.isFinite(readTs) || bumpTs > readTs)
        ) {
          scheduleUnreadRefetch();
          return;
        }
        delete lastDmBumpAt[peerId];
        useUnreadStore.getState().resetDm(peerId);
        return;
      }
      if (readerId && readAt) {
        useReadReceiptStore.getState().setDmPeerReadAt(readerId, readAt);
      }
      return;
  },
};
