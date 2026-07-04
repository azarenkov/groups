import { ws, WsEnvelope } from "./websocket";
import { useServerStore } from "../store/serverStore";
import { useChannelStore } from "../store/channelStore";
import { useFriendshipStore } from "../store/friendshipStore";
import { useVoicePresenceStore } from "../store/voicePresenceStore";
import { useStreamViewersStore } from "../store/streamViewersStore";
import { useCallStore } from "../store/callStore";
import { useUnreadStore } from "../store/unreadStore";
import { useReadReceiptStore } from "../store/readReceiptStore";
import { usePresenceStore } from "../store/presenceStore";
import { useAuthStore } from "../store/authStore";
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

function asString(v: unknown): string | undefined {
  return typeof v === "string" ? v : undefined;
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

    case "server.deleted": {
      const id = asString(d.id);
      if (!id) return;
      const serverState = useServerStore.getState();
      const channelState = useChannelStore.getState();
      const voiceState = useVoicePresenceStore.getState();
      const unreadState = useUnreadStore.getState();
      const channelIds = (channelState.channelsByServer[id] ?? []).map((c) => c.id);
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
    case "friendship.requestAccepted":
    case "friendship.removed": {
      void useFriendshipStore.getState().fetchFriends();
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
      const byServer = useChannelStore.getState().channelsByServer;
      let serverId: string | undefined;
      for (const [sid, chans] of Object.entries(byServer)) {
        if (chans.some((c) => c.id === channelId)) {
          serverId = sid;
          break;
        }
      }
      useUnreadStore.getState().bumpChannel(channelId, serverId);
      return;
    }

    case "dm_message.sent": {
      const senderId = asString(d.senderId);
      const recipientId = asString(d.recipientId);
      const selfId = useAuthStore.getState().user?.id;
      if (!senderId || !recipientId || !selfId) return;
      if (recipientId !== selfId) return;
      if (activeDmView === senderId) {
        throttleMarkDmRead(senderId);
        return;
      }
      useUnreadStore.getState().bumpDm(senderId);
      return;
    }

    case "dm.read": {
      const readerId = asString(d.readerId);
      const readAt = asString(d.readAt);
      if (readerId && readAt) {
        useReadReceiptStore.getState().setDmPeerReadAt(readerId, readAt);
      }
      return;
    }

    case "channel.read": {
      const channelId = asString(d.channelId);
      const readerId = asString(d.readerId);
      const readAt = asString(d.readAt);
      if (channelId && readerId && readAt) {
        useReadReceiptStore
          .getState()
          .setChannelMemberRead(channelId, readerId, readAt);
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
        .setChannelMute(channelId, userId, isMuted);
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

export function installWsBootstrap() {
  if (installed) return;
  installed = true;
  ws.on(dispatch);
  ws.onOpen(() => {
    void useCallStore.getState().rehydrate();
  });
}
