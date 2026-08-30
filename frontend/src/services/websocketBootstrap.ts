import { ws, WsEnvelope } from "./websocket";
import { useServerStore } from "../store/serverStore";
import { useChannelStore } from "../store/channelStore";
import { useFriendshipStore } from "../store/friendshipStore";
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
import { useAuthStore } from "../store/authStore";
import { useUserNamesStore } from "../store/userNamesStore";
import {
  channelNotifLevel,
  dmNotifLevel,
  useNotificationPrefsStore,
} from "../store/notificationPrefsStore";
import { mentionsUser } from "../utils/mentions";
import { playNotificationSound } from "../utils/notificationSound";
import type { Server } from "../types/server";
import type { Channel } from "../types/channel";

let activeChannelView: string | null = null;
let activeDmView: string | null = null;

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
      return;
    }

    case "server.memberLeft": {
      const serverId = asString(d.serverId);
      const userId = asString(d.userId);
      if (!serverId || !userId) return;
      const me = useAuthStore.getState().user;
      if (me && me.id === userId) {
        purgeServerLocally(serverId);
        return;
      }
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
      void useFriendshipStore.getState().fetchFriends();
      return;
    }

    case "friendship.removed": {
      void useFriendshipStore.getState().fetchFriends();
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
      window.dispatchEvent(
        new CustomEvent("rift:profile-changed", {
          detail: { userId, bio: asString(d.bio) ?? "", showActivity: d.showActivity === true },
        }),
      );
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
        void useFriendshipStore.getState().fetchFriends();
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
      if (activeChannelView === channelId) {
        throttleMarkChannelRead(channelId);
        return;
      }
      const level = channelNotifLevel(channelId);
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
      for (const [sid, chans] of Object.entries(byServer)) {
        if (chans.some((c) => c.id === channelId)) {
          serverId = sid;
          break;
        }
      }
      lastChannelBumpAt[channelId] = Math.max(
        lastChannelBumpAt[channelId] ?? 0,
        eventTs(d.occurredAt),
      );
      useUnreadStore.getState().bumpChannel(channelId, serverId);
      playNotificationSound();
      return;
    }

    case "message.edited": {
      const channelId = asString(d.channelId);
      if (!channelId) return;
      if (activeChannelView === channelId) return;
      if (channelNotifLevel(channelId) !== "mentions") return;
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
      if (activeDmView === senderId) {
        throttleMarkDmRead(senderId);
        return;
      }
      if (dmNotifLevel(senderId) === "none") return;
      lastDmBumpAt[senderId] = Math.max(
        lastDmBumpAt[senderId] ?? 0,
        eventTs(d.occurredAt),
      );
      useUnreadStore.getState().bumpDm(senderId);
      playNotificationSound();
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
      if (userId) usePresenceStore.getState().setOnline(userId);
      return;
    }

    case "user.offline": {
      const userId = asString(d.userId);
      if (userId) usePresenceStore.getState().setOffline(userId);
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
  ws.on(dispatch);
  ws.onOpen(() => {
    const gen = ++prefsReloadGen;
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
    void useFriendshipStore
      .getState()
      .fetchFriends()
      .then(() => {
        if (useFriendshipStore.getState().error === null) {
          useUserNamesStore.getState().clear();
        }
      });
  });
}
