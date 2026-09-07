import { ws, WsEnvelope } from "./websocket";
import { useServerStore } from "../store/serverStore";
import { useChannelStore } from "../store/channelStore";
import { useFriendshipStore } from "../store/friendshipStore";
import { profileVersion, useUserProfileStore } from "../store/userProfileStore";
import { scheduleHomeRefresh, useHomeStore } from "../store/homeStore";
import { useVoicePresenceStore } from "../store/voicePresenceStore";
import { useStreamViewersStore } from "../store/streamViewersStore";
import { useCallStore } from "../store/callStore";
import { useUnreadStore } from "../store/unreadStore";
import { useReadReceiptStore } from "../store/readReceiptStore";
import {
  refetchDmPreviews,
  useDmPreviewStore,
} from "../store/dmPreviewStore";
import { usePresenceStore } from "../store/presenceStore";
import {
  loadActivitiesFor,
  loadFriendActivities,
  useActivityStore,
} from "../store/activityStore";
import { useAuthStore } from "../store/authStore";
import { useUserNamesStore } from "../store/userNamesStore";
import {
  markNotificationPrefsStale,
  notificationPrefsLoaded,
  useNotificationPrefsStore,
} from "../store/notificationPrefsStore";
import {
  defaultMentionLevel,
  desktopNotifAllowed,
  messageSoundAllowed,
  useNotificationSettingsStore,
} from "../store/notificationSettingsStore";
import { showDesktopNotification } from "../utils/desktopNotification";
import { mentionsUser } from "../utils/mentions";
import { playNotificationSound } from "../utils/notificationSound";
import type { Server } from "../types/server";
import type { Channel } from "../types/channel";

let activeChannelView: string | null = null;
let activeDmView: string | null = null;

const viewIsForeground = () =>
  typeof document === "undefined" ||
  (document.visibilityState === "visible" && document.hasFocus());

export function setActiveChatView(view:
  | { kind: "channel"; id: string }
  | { kind: "dm"; id: string }
  | null) {
  activeChannelView = view?.kind === "channel" ? view.id : null;
  activeDmView = view?.kind === "dm" ? view.id : null;
}

const READ_MARK_THROTTLE_MS = 1500;
const lastChannelReadAt: Record<string, number> = {};
const lastDmReadAt: Record<string, number> = {};

function throttleMarkChannelRead(channelId: string) {
  const now = Date.now();
  if (now - (lastChannelReadAt[channelId] ?? 0) < READ_MARK_THROTTLE_MS) return;
  lastChannelReadAt[channelId] = now;
  void import("../api/user").then(({ userAPI }) =>
    userAPI.markChannelRead(channelId).catch(() => {
      lastChannelReadAt[channelId] = 0;
    }),
  );
}

function throttleMarkDmRead(friendId: string) {
  const now = Date.now();
  if (now - (lastDmReadAt[friendId] ?? 0) < READ_MARK_THROTTLE_MS) return;
  lastDmReadAt[friendId] = now;
  void import("../api/user").then(({ userAPI }) =>
    userAPI.markDmRead(friendId).catch(() => {
      lastDmReadAt[friendId] = 0;
    }),
  );
}

let installed = false;
let prefsReloadGen = 0;

function asString(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
}

const UNREAD_REFETCH_DELAY_MS = 1000;
let unreadRefetchTimer: ReturnType<typeof setTimeout> | null = null;

const lastChannelBumpAt: Record<string, number> = {};
const lastDmBumpAt: Record<string, number> = {};

function eventTs(v: unknown): number {
  const t = Date.parse(asString(v) ?? "");
  return Number.isFinite(t) ? t : Date.now();
}

function scheduleUnreadRefetch() {
  if (unreadRefetchTimer) return;
  unreadRefetchTimer = setTimeout(() => {
    unreadRefetchTimer = null;
    void import("../store/unreadStore").then(({ refetchUnread }) =>
      refetchUnread().catch(() => {}),
    );
  }, UNREAD_REFETCH_DELAY_MS);
}

function dispatch(env: WsEnvelope) {
  const d = env.data as Record<string, unknown>;
  switch (env.type) {
    case "server.created": {
      const id = asString(d.id);
      const ownerId = asString(d.ownerId);
      const name = asString(d.name) ?? "";
      if (!id || !ownerId) return;
      const server: Server = {
        id,
        name,
        ownerId,
        description: (asString(d.description) ?? null) as string | null,
        accentColor: null,
        createdAt: asString(d.occurredAt) ?? new Date().toISOString(),
        updatedAt: asString(d.occurredAt) ?? new Date().toISOString(),
      };
      useServerStore.getState().addServer(server);
      return;
    }

    case "server.ownerChanged": {
      const id = asString(d.id);
      const ownerId = asString(d.ownerId);
      if (!id || !ownerId) return;
      useServerStore.getState().patchServer(id, { ownerId });
      return;
    }

    case "server.updated": {
      const id = asString(d.id);
      if (!id) return;
      const patch: { name?: string; description?: string | null } = {};
      const name = asString(d.name);
      if (name) patch.name = name;
      patch.description = asString(d.description) ?? null;
      useServerStore.getState().patchServer(id, patch);
      return;
    }

    case "server.deleted": {
      const id = asString(d.id);
      if (!id) return;
      purgeServerLocally(id);
      void loadActivitiesFor(Object.keys(useActivityStore.getState().byUser));
      return;
    }

    case "server.memberLeft": {
      const serverId = asString(d.serverId);
      const userId = asString(d.userId);
      if (!serverId || !userId) return;
      const me = useAuthStore.getState().user;
      if (me && me.id === userId) {
        purgeServerLocally(serverId);
        void loadActivitiesFor(Object.keys(useActivityStore.getState().byUser));
        return;
      }
      useActivityStore.getState().apply(userId, null);
      void loadActivitiesFor([userId]);
      const channels = useChannelStore.getState().channelsByServer[serverId] ?? [];
      const receipts = useReadReceiptStore.getState();
      for (const c of channels) {
        receipts.removeChannelMember(c.id, userId);
      }
      useVoicePresenceStore
        .getState()
        .purgePendingMutes(channels.map((c) => c.id), userId, serverId);
      window.dispatchEvent(
        new CustomEvent("rift:server-members-changed", { detail: { serverId } }),
      );
      void import("../store/rolesStore").then(({ useRolesStore }) => {
        const roles = useRolesStore.getState();
        roles.removeMember(serverId, userId);
        if (roles.byServer[serverId]) {
          void roles.fetch(serverId).catch(() => {});
        }
      });
      return;
    }

    case "channel.created": {
      const id = asString(d.id);
      const serverId = asString(d.serverId);
      const ownerId = asString(d.ownerId);
      const name = asString(d.name) ?? "";
      const kind = (d.kind === "voice" ? "voice" : "text") as Channel["kind"];
      const visibility = (d.visibility === "private"
        ? "private"
        : "public") as Channel["visibility"];
      if (!id || !serverId || !ownerId) return;
      const channel: Channel = {
        id,
        name,
        ownerId,
        kind,
        visibility,
        maxParticipants: 0,
        createdAt: asString(d.occurredAt) ?? new Date().toISOString(),
      };
      useChannelStore.getState().addChannel(serverId, channel);
      return;
    }

    case "channel.deleted": {
      const id = asString(d.id);
      const serverId = asString(d.serverId);
      if (!id || !serverId) return;
      useChannelStore.getState().removeChannel(serverId, id);
      useVoicePresenceStore.getState().clearChannel(id);
      useUnreadStore.getState().resetChannel(id);
      return;
    }

    case "channel.renamed": {
      const id = asString(d.id);
      const serverId = asString(d.serverId);
      const newName = asString(d.newName);
      if (!id || !serverId || !newName) return;
      useChannelStore.getState().renameChannel(serverId, id, newName);
      return;
    }

    case "friendship.requestSent":
    case "friendship.requestAccepted": {
      void useFriendshipStore
        .getState()
        .fetchFriends()
        .then(() => loadFriendActivities());
      return;
    }

    case "friendship.removed": {
      const removedSelf = useAuthStore.getState().user?.id;
      const removedA = asString(d.userId);
      const removedB = asString(d.otherId);
      const peer = removedA === removedSelf ? removedB : removedA;
      if (peer) {
        useActivityStore.getState().apply(peer, null);
        void loadActivitiesFor([peer]);
      }
      void useFriendshipStore
        .getState()
        .fetchFriends()
        .then(() => loadFriendActivities());
      refetchDmPreviews();
      return;
    }

    case "server.memberJoined": {
      const userId = asString(d.userId);
      const userName = asString(d.userName);
      const userNameUpdatedAt = asString(d.userNameUpdatedAt);
      if (!userId || !userName || !userNameUpdatedAt) return;
      const applied = useUserNamesStore
        .getState()
        .applyRename(userId, userName, userNameUpdatedAt);
      if (applied) {
        useVoicePresenceStore.getState().renameMember(userId, userName);
        useCallStore.getState().participantRenamed(userId, userName);
      }
      const joinedServerId = asString(d.serverId);
      if (joinedServerId) {
        window.dispatchEvent(
          new CustomEvent("rift:server-members-changed", {
            detail: { serverId: joinedServerId },
          }),
        );
      }
      return;
    }

    case "user.profileChanged": {
      const userId = asString(d.userId);
      if (!userId) return;
      const bio = asString(d.bio) ?? "";
      const showActivity = d.showActivity === true;
      const showGameActivity = d.showGameActivity !== false;
      const version = profileVersion(asString(d.occurredAt));
      const cached = useUserProfileStore.getState().bios[userId];
      if (cached && cached.at > version) return;
      useUserProfileStore.getState().apply(userId, bio, version);
      const auth = useAuthStore.getState();
      if (!showGameActivity && auth.user?.id !== userId) {
        useActivityStore.getState().apply(userId, null);
      }
      if (auth.user?.id === userId) {
        auth.setUser({ ...auth.user, bio, showActivity, showGameActivity });
      } else if (!showActivity) {
        useHomeStore.getState().hideFriendActivity(userId, version);
        scheduleHomeRefresh();
      } else {
        useHomeStore.getState().unhideFriendActivity(userId, version);
        scheduleHomeRefresh();
      }
      return;
    }

    case "user.renamed": {
      const userId = asString(d.userId);
      const newName = asString(d.name);
      if (!userId) return;
      if (newName) {
        const occurredAt = asString(d.occurredAt);
        const applied = useUserNamesStore
          .getState()
          .applyRename(userId, newName, occurredAt);
        if (!applied) return;
        useVoicePresenceStore.getState().renameMember(userId, newName);
        useCallStore.getState().participantRenamed(userId, newName);
      }
      const me = useAuthStore.getState().user;
      if (me && me.id === userId) {
        void useAuthStore.getState().fetchMe();
      } else {
        void useFriendshipStore
        .getState()
        .fetchFriends()
        .then(() => loadFriendActivities());
      }
      return;
    }

    case "channel.participant.joined": {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const userName = asString(d.userName) ?? userId;
      useVoicePresenceStore.getState().addMember(channelId, {
        id: userId,
        name: userName,
        isMuted: false,
        cameraOn: false,
        screenOn: false,
      });
      return;
    }

    case "channel.participant.left":
    case "channel.participant.connectionAborted": {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      useVoicePresenceStore.getState().removeMember(channelId, userId);
      return;
    }

    case "message.sent": {
      const channelId = asString(d.channelId);
      const authorId = asString(d.authorId);
      const selfId = useAuthStore.getState().user?.id;
      if (!channelId || !authorId || authorId === selfId) return;
      if (activeChannelView === channelId && viewIsForeground()) {
        throttleMarkChannelRead(channelId);
        return;
      }
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
      if (messageSoundAllowed()) playNotificationSound();
      if (desktopNotifAllowed()) {
        showDesktopNotification(
          channelName ? `#${channelName}` : "Rift",
          asString(d.content) ?? "",
        );
      }
      return;
    }

    case "message.edited": {
      const channelId = asString(d.channelId);
      if (!channelId) return;
      if (activeChannelView === channelId && viewIsForeground()) return;
      const editLevel =
        useNotificationPrefsStore.getState().channelLevels[channelId] ??
        defaultMentionLevel();
      if (editLevel !== "mentions") return;
      scheduleUnreadRefetch();
      return;
    }

    case "dm_message.sent": {
      const senderId = asString(d.senderId);
      const recipientId = asString(d.recipientId);
      const selfId = useAuthStore.getState().user?.id;
      if (!senderId || !recipientId || !selfId) return;
      const peerId = senderId === selfId ? recipientId : senderId;
      const peerName =
        (senderId !== selfId ? asString(d.senderName) : undefined) ??
        useFriendshipStore.getState().friends.find((f) => f.friendId === peerId)?.friendName;
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
      if (recipientId !== selfId) return;
      if (activeDmView === senderId && viewIsForeground()) {
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
    }

    case "dm_message.edited": {
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
    }

    case "dm_message.deleted": {
      const id = asString(d.id);
      if (id && useDmPreviewStore.getState().hasMessage(id)) {
        refetchDmPreviews(true);
      }
      return;
    }

    case "dm.read": {
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
    }

    case "channel.read": {
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
    }

    case "server.roles.changed": {
      const serverId = asString(d.serverId);
      if (!serverId) return;
      void import("../store/rolesStore").then(({ useRolesStore }) => {
        const roles = useRolesStore.getState();
        if (roles.byServer[serverId] || roles.isTracked(serverId)) {
          void roles.fetch(serverId).catch(() => {});
        }
      });
      return;
    }

    case "notification_preference.updated": {
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
    }

    case "channel.text.joined": {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      const selfId = useAuthStore.getState().user?.id;
      if (channelId && userId && userId !== selfId) {
        useReadReceiptStore.getState().addChannelMember(channelId, userId);
      }
      return;
    }

    case "user.online": {
      const userId = asString(d.userId);
      if (userId) {
        usePresenceStore.getState().setOnline(userId);
        void loadActivitiesFor([userId]);
      }
      return;
    }

    case "user.offline": {
      const userId = asString(d.userId);
      if (userId) {
        usePresenceStore.getState().setOffline(userId);
        useActivityStore.getState().apply(userId, null);
      }
      return;
    }

    case "user.activityChanged": {
      const userId = asString(d.userId);
      if (!userId) return;
      const gameId = asString(d.gameId);
      const name = asString(d.name);
      const startedAt = asString(d.startedAt);
      const occurredAt = asString(d.occurredAt) ?? null;
      useActivityStore
        .getState()
        .applyEvent(
          userId,
          gameId && name && startedAt ? { gameId, name, startedAt } : null,
          occurredAt,
        );
      return;
    }

    case "call.incoming": {
      const callId = asString(d.callId);
      const callerId = asString(d.callerId);
      if (!callId || !callerId) return;
      const callerName = asString(d.callerName) ?? callerId;
      useCallStore.getState().setIncoming({ callId, callerId, callerName });
      return;
    }

    case "call.participant.joined": {
      const callId = asString(d.callId);
      const userId = asString(d.userId);
      if (!callId || !userId) return;
      const userName = asString(d.userName) ?? userId;
      useCallStore.getState().participantJoined(callId, userId, userName);
      return;
    }

    case "call.participant.left": {
      const callId = asString(d.callId);
      const userId = asString(d.userId);
      if (!callId || !userId) return;
      useCallStore.getState().participantLeft(callId, userId);
      return;
    }

    case "call.declined": {
      const callId = asString(d.callId);
      const userId = asString(d.userId);
      if (callId && userId) {
        useCallStore.getState().participantDeclined(callId, userId);
      }
      return;
    }

    case "call.available": {
      const callId = asString(d.callId);
      const userId = asString(d.userId);
      if (callId && userId) {
        useCallStore.getState().callAvailable(userId, callId);
      }
      return;
    }

    case "call.ended": {
      const callId = asString(d.callId);
      if (callId) useCallStore.getState().callEnded(callId);
      return;
    }

    case "channel.participant.muted":
    case "channel.participant.unmuted": {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const isMuted = env.type === "channel.participant.muted";
      useVoicePresenceStore
        .getState()
        .setChannelMute(
          channelId,
          userId,
          isMuted,
          isMuted ? (asString(d.mutedBy) ?? undefined) : undefined,
          asString(d.serverId) ?? undefined,
        );
      const selfId = useAuthStore.getState().user?.id;
      const mutedBy = asString(d.mutedBy) ?? asString(d.unmutedBy);
      if (userId === selfId && mutedBy && mutedBy !== selfId) {
        window.dispatchEvent(
          new CustomEvent(
            isMuted ? "rift:moderator-mute" : "rift:moderator-unmute",
            { detail: { channelId, mutedBy } },
          ),
        );
      }
      return;
    }

    case "channel.participant.camera.on":
    case "channel.participant.camera.off":
    case "channel.participant.screen.on":
    case "channel.participant.screen.off": {
      const channelId = asString(d.channelId);
      const userId = asString(d.userId);
      if (!channelId || !userId) return;
      const source = env.type.includes("camera") ? "camera" : "screen";
      const enabled = env.type.endsWith(".on");
      useVoicePresenceStore
        .getState()
        .setChannelMedia(channelId, userId, source, enabled);
      return;
    }

    case "channel.stream.viewers": {
      const serverId = asString(d.serverId);
      const channelId = asString(d.channelId);
      const streamerId = asString(d.streamerId);
      if (!serverId || !channelId || !streamerId) return;
      const count = typeof d.count === "number" ? d.count : 0;
      useStreamViewersStore
        .getState()
        .setCount(serverId, channelId, streamerId, count);
      return;
    }

    default:
      return;
  }
}

function purgeServerLocally(id: string) {
  void import("../store/rolesStore").then(({ useRolesStore }) => {
    useRolesStore.getState().clearServer(id);
  });
  const serverState = useServerStore.getState();
  const channelState = useChannelStore.getState();
  const voiceState = useVoicePresenceStore.getState();
  const unreadState = useUnreadStore.getState();
  const channelIds = (channelState.channelsByServer[id] ?? []).map((c) => c.id);
  voiceState.purgePendingForServer(id, channelIds);
  for (const cid of channelIds) {
    voiceState.clearChannel(cid);
    unreadState.resetChannel(cid);
  }
  if (
    channelState.selectedChannelId &&
    channelIds.includes(channelState.selectedChannelId)
  ) {
    channelState.setSelectedChannel(null);
  }
  channelState.clearChannels(id);
  if (serverState.selectedServerId === id) {
    serverState.setSelectedServer(null);
  }
  serverState.removeServer(id);
}

export function installWsBootstrap() {
  if (installed) return;
  installed = true;
  const onForeground = () => {
    if (!viewIsForeground()) return;
    if (activeChannelView) {
      lastChannelReadAt[activeChannelView] = 0;
      throttleMarkChannelRead(activeChannelView);
    }
    if (activeDmView) {
      lastDmReadAt[activeDmView] = 0;
      throttleMarkDmRead(activeDmView);
    }
    if (activeChannelView || activeDmView) scheduleUnreadRefetch();
  };
  window.addEventListener("focus", onForeground);
  document.addEventListener("visibilitychange", onForeground);
  ws.on(dispatch);
  ws.onOpen(() => {
    const gen = ++prefsReloadGen;
    markNotificationPrefsStale();
    const reconnectUserId = useAuthStore.getState().user?.id;
    if (reconnectUserId) {
      void useNotificationSettingsStore.getState().syncFromServer(reconnectUserId);
    }
    void (async () => {
      let delayMs = 1000;
      for (;;) {
        try {
          await useNotificationPrefsStore.getState().load();
          break;
        } catch (err) {
          console.warn("[ws] failed to reload notification prefs:", err);
          const wait = delayMs;
          await new Promise((r) => setTimeout(r, wait));
          delayMs = Math.min(delayMs * 2, 15000);
          if (gen !== prefsReloadGen) return;
        }
      }
      if (gen !== prefsReloadGen) return;
      scheduleUnreadRefetch();
    })();
    void import("../store/rolesStore").then(({ useRolesStore }) => {
      const roles = useRolesStore.getState();
      const tracked = new Set([
        ...Object.keys(roles.byServer),
        ...roles.trackedServers(),
      ]);
      for (const serverId of tracked) {
        void roles.fetch(serverId).catch(() => {});
      }
    });
    void useCallStore.getState().rehydrate();
    useUserNamesStore.getState().bumpSyncTick();
    const friendsBefore = useFriendshipStore
      .getState()
      .friends.filter((f) => f.status === "accepted")
      .map((f) => f.friendId);
    void useFriendshipStore
      .getState()
      .fetchFriends()
      .then(() => {
        if (useFriendshipStore.getState().error === null) {
          useUserNamesStore.getState().clear();
          const after = new Set(
            useFriendshipStore
              .getState()
              .friends.filter((f) => f.status === "accepted")
              .map((f) => f.friendId),
          );
          const gone = friendsBefore.filter((id) => !after.has(id));
          for (const id of gone) useActivityStore.getState().apply(id, null);
          if (gone.length > 0) void loadActivitiesFor(gone);
          void loadFriendActivities();
        }
      });
  });
}
