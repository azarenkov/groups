import type { WsHandlers } from "../../../shared/ws/types";
import { useAuthStore } from "../../auth/store/authStore";
import { isBlockedEitherWay } from "../../social/store/blockStore";
import { refetchDmPreviews, useDmPreviewStore } from "../store/dmPreviewStore";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { notificationPrefsLoaded, useNotificationPrefsStore } from "../../settings/store/notificationPrefsStore";
import { defaultMentionLevel, desktopNotifAllowed, messageSoundAllowed } from "../../settings/store/notificationSettingsStore";
import { useReadReceiptStore } from "../../chat/store/readReceiptStore";
import { useUnreadStore } from "../../chat/store/unreadStore";
import { useUserNamesStore } from "../../social/store/userNamesStore";
import { showDesktopNotification } from "../../../shared/lib/desktopNotification";
import { playNotificationSound } from "../../../shared/lib/notificationSound";
import { voiceDurationFromPayload } from "../../chat/lib/voice";
import { notificationBody } from "../../chat/lib/notificationBody";
import { asString, eventTs, getActiveDmView, lastDmBumpAt, scheduleUnreadRefetch, throttleMarkDmRead, viewIsForeground } from "../../../shared/ws/context";

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
        inviteCommunityId: asString(d.inviteCommunityId) ?? null,
        hasAttachments: Array.isArray(d.attachments) && d.attachments.length > 0,
        voiceDurationMs: voiceDurationFromPayload(d.attachments),
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
        showDesktopNotification(peerName ?? "Rift", notificationBody(asString(d.content), d.attachments));
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
  "message.hidden": (d) => {
      const id = asString(d.messageId);
      if (id && useDmPreviewStore.getState().hasMessage(id)) {
        refetchDmPreviews(true);
      }
      void import("../../chat/store/unreadStore").then(({ refetchUnread }) =>
        refetchUnread().catch(() => {}),
      );
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
