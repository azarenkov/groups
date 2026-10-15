import type { WsHandlers } from "../../../shared/ws/types";
import { useAuthStore } from "../../auth/store/authStore";
import { isBlockedEitherWay } from "../../social/store/blockStore";
import { refetchDmPreviews, useDmPreviewStore } from "../store/dmPreviewStore";
import { useFriendshipStore } from "../../social/store/friendshipStore";
import { notificationPrefsLoaded, useNotificationPrefsStore } from "../../settings/store/notificationPrefsStore";
import { defaultMentionLevel, desktopNotifAllowed, messageSoundAllowed } from "../../settings/store/notificationSettingsStore";
import { useReadReceiptStore } from "../../chat/store/readReceiptStore";
import { refetchUnread, useUnreadStore } from "../../chat/store/unreadStore";
import { useUserNamesStore } from "../../social/store/userNamesStore";
import { showDesktopNotification } from "../../../shared/lib/desktopNotification";
import { playNotificationSound } from "../../../shared/lib/notificationSound";
import { videoNoteDurationFromPayload, voiceDurationFromPayload } from "../../chat/lib/voice";
import { notificationBody } from "../../chat/lib/notificationBody";
import { countsAsUnread } from "../../voice/lib/callCard";
import { useCallStore } from "../../voice/store/callStore";
import { hiddenDmsKnown, isDmHidden, useHiddenDmStore } from "../store/hiddenDmStore";
import { clearedBefore, dmClearedFor } from "../lib/dmCleared";
import type { PinnedMessage } from "../../chat/api/pins";
import type { InfiniteData } from "@tanstack/react-query";
import type { GlobalSearchPage } from "../../search/api/globalSearch";
import { queryClient, queryKeys } from "../../../shared/query/queryClient";
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
        hasAttachments: Array.isArray(d.attachments) && d.attachments.length > 0,
        voiceDurationMs: voiceDurationFromPayload(d.attachments),
        videoNoteDurationMs: videoNoteDurationFromPayload(d.attachments),
        stickerEmoji: asString((d.sticker as { emoji?: unknown } | null | undefined)?.emoji) ?? null,
        createdAt: asString(d.occurredAt) ?? new Date().toISOString(),
      });
      if (!knownPeerName) refetchDmPreviews();
      if (recipientId !== selfId) return;
      const callStatus = asString(d.callStatus);
      if (!countsAsUnread(callStatus)) return;
      if (getActiveDmView() === senderId && viewIsForeground()) {
        throttleMarkDmRead(senderId);
        return;
      }
      if (!notificationPrefsLoaded()) {
        scheduleUnreadRefetch();
        return;
      }
      const alert = () => {
        const dmLevel =
          useNotificationPrefsStore.getState().dmLevels[senderId] ??
          (defaultMentionLevel() === "none" ? "none" : "all");
        if (dmLevel === "none" || !hiddenDmsKnown() || isDmHidden(senderId)) return;
        if (callStatus) useCallStore.getState().setCallNotice({ kind: "missed", userId: senderId, name: peerName });
        if (messageSoundAllowed()) playNotificationSound();
        if (desktopNotifAllowed()) {
          showDesktopNotification(peerName ?? "Rift", notificationBody(asString(d.content), d.attachments, d.sticker, callStatus));
        }
      };
      if (callStatus) {
        const before = useUnreadStore.getState().byDm[senderId] ?? 0;
        refetchUnread().then(
          () => {
            if ((useUnreadStore.getState().byDm[senderId] ?? 0) > before) alert();
          },
          () => {},
        );
        return;
      }
      lastDmBumpAt[senderId] = Math.max(
        lastDmBumpAt[senderId] ?? 0,
        eventTs(d.occurredAt),
      );
      useUnreadStore.getState().bumpDm(senderId);
      alert();
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
      const senderId = asString(d.senderId);
      if (
        senderId &&
        asString(d.recipientId) === useAuthStore.getState().user?.id &&
        (useUnreadStore.getState().byDm[senderId] ?? 0) > 0
      ) {
        void refetchUnread().catch(() => {});
      }
      return;
  },
  "message.hidden": (d) => {
      const id = asString(d.messageId);
      if (id && useDmPreviewStore.getState().hasMessage(id)) {
        refetchDmPreviews(true);
      }
      void refetchUnread().catch(() => {});
      return;
  },
  "dm_conversation.cleared": (d) => {
      const cleared = dmClearedFor(d, useAuthStore.getState().user?.id);
      if (!cleared) return;
      const bumpTs = lastDmBumpAt[cleared.peerId];
      if (bumpTs === undefined || bumpTs <= Date.parse(cleared.clearedAt)) {
        delete lastDmBumpAt[cleared.peerId];
        useUnreadStore.getState().resetDm(cleared.peerId);
      }
      useDmPreviewStore.getState().dropCleared(cleared.peerId, cleared.clearedAt);
      refetchDmPreviews(true);
      queryClient.setQueryData<PinnedMessage[]>(queryKeys.pins(`dm:${cleared.peerId}`), (prev: PinnedMessage[] | undefined) =>
        prev ? prev.filter((p) => !clearedBefore(p.createdAt, cleared.clearedAt)) : prev,
      );
      void queryClient.invalidateQueries({ queryKey: queryKeys.pins(`dm:${cleared.peerId}`) });
      queryClient.setQueriesData<InfiniteData<GlobalSearchPage>>({ queryKey: ["globalSearch"] }, (prev: InfiniteData<GlobalSearchPage> | undefined) =>
        prev
          ? {
              ...prev,
              pages: prev.pages.map((page) => ({
                ...page,
                items: page.items.filter(
                  (hit) =>
                    !(
                      hit.place.type === "dm" &&
                      hit.place.peerId === cleared.peerId &&
                      clearedBefore(hit.createdAt, cleared.clearedAt)
                    ),
                ),
              })),
            }
          : prev,
      );
      void queryClient.invalidateQueries({ queryKey: ["globalSearch"] });
      void refetchUnread().catch(() => {});
      return;
  },
  "dm_conversation.hidden_changed": (d) => {
      const peerId = asString(d.peerId);
      if (!peerId) return;
      useHiddenDmStore.getState().remoteChanged(peerId, d.hidden === true);
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
