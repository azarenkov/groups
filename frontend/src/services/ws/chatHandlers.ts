import type { WsHandlers } from "./types";
import { useAuthStore } from "../../store/authStore";
import { useBlockStore } from "../../store/blockStore";
import { useChannelStore } from "../../store/channelStore";
import { notificationPrefsLoaded, useNotificationPrefsStore } from "../../store/notificationPrefsStore";
import { defaultMentionLevel, desktopNotifAllowed, messageSoundAllowed } from "../../store/notificationSettingsStore";
import { useReadReceiptStore } from "../../store/readReceiptStore";
import { useUnreadStore } from "../../store/unreadStore";
import { showDesktopNotification } from "../../utils/desktopNotification";
import { mentionsUser } from "../../utils/mentions";
import { playNotificationSound } from "../../utils/notificationSound";
import { asString, eventTs, getActiveChannelView, lastChannelBumpAt, scheduleUnreadRefetch, throttleMarkChannelRead, viewIsForeground } from "./context";

export const chatHandlers: WsHandlers = {
  "message.sent": (d) => {
      const channelId = asString(d.channelId);
      const authorId = asString(d.authorId);
      const selfId = useAuthStore.getState().user?.id;
      if (!channelId || !authorId || authorId === selfId) return;
      if (getActiveChannelView() === channelId && viewIsForeground()) {
        throttleMarkChannelRead(channelId);
        return;
      }
      if (!useBlockStore.getState().loaded) {
        scheduleUnreadRefetch();
        return;
      }
      if (useBlockStore.getState().blocked[authorId]) return;
      if (!notificationPrefsLoaded()) {
        scheduleUnreadRefetch();
        return;
      }
      const level =
        useNotificationPrefsStore.getState().channelLevels[channelId] ??
        defaultMentionLevel();
      if (level === "none") return;
      if (level === "mentions") {
        const myName = useAuthStore.getState().user?.name;
        const mentioned = Array.isArray(d.mentions)
          ? !!selfId && d.mentions.includes(selfId)
          : !!myName && mentionsUser(asString(d.content) ?? "", myName);
        if (!mentioned) return;
      }
      const byServer = useChannelStore.getState().channelsByServer;
      let serverId: string | undefined;
      let channelName: string | undefined;
      for (const [sid, chans] of Object.entries(byServer)) {
        const found = chans.find((c) => c.id === channelId);
        if (found) {
          serverId = sid;
          channelName = found.name;
          break;
        }
      }
      lastChannelBumpAt[channelId] = Math.max(
        lastChannelBumpAt[channelId] ?? 0,
        eventTs(d.occurredAt),
      );
      useUnreadStore.getState().bumpChannel(channelId, serverId);
      if (asString(d.kind) === "system") return;
      if (messageSoundAllowed()) playNotificationSound();
      if (desktopNotifAllowed()) {
        showDesktopNotification(
          channelName ? `#${channelName}` : "Rift",
          asString(d.content) ?? "",
        );
      }
      return;
  },
  "message.edited": (d) => {
      const channelId = asString(d.channelId);
      if (!channelId) return;
      if (getActiveChannelView() === channelId && viewIsForeground()) return;
      const editLevel =
        useNotificationPrefsStore.getState().channelLevels[channelId] ??
        defaultMentionLevel();
      if (editLevel !== "mentions") return;
      scheduleUnreadRefetch();
      return;
  },
  "channel.read": (d) => {
      const channelId = asString(d.channelId);
      const readerId = asString(d.readerId);
      const readAt = asString(d.readAt);
      const selfId = useAuthStore.getState().user?.id;
      if (channelId && readerId && readerId === selfId) {
        const readTs = Date.parse(readAt ?? "");
        const bumpTs = lastChannelBumpAt[channelId];
        if (
          bumpTs !== undefined &&
          (!Number.isFinite(readTs) || bumpTs > readTs)
        ) {
          scheduleUnreadRefetch();
          return;
        }
        delete lastChannelBumpAt[channelId];
        useUnreadStore.getState().resetChannel(channelId);
        return;
      }
      if (channelId && readerId && readAt) {
        useReadReceiptStore
          .getState()
          .setChannelMemberRead(channelId, readerId, readAt);
      }
      return;
  },
  "notification_preference.updated": (d) => {
      const scopeType = asString(d.scopeType);
      const scopeId = asString(d.scopeId);
      const level = asString(d.level);
      if (!scopeType || !scopeId || !level) return;
      useNotificationPrefsStore
        .getState()
        .applyRemote(
          scopeType,
          scopeId,
          level,
          typeof d.version === "number" ? d.version : undefined,
        );
      scheduleUnreadRefetch();
      return;
  },
};
