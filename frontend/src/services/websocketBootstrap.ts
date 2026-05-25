import { ws, WsEnvelope } from "./websocket";
import { useServerStore } from "../store/serverStore";
import { useChannelStore } from "../store/channelStore";
import { useFriendshipStore } from "../store/friendshipStore";
import { useVoicePresenceStore } from "../store/voicePresenceStore";
import type { Server } from "../types/server";
import type { Channel } from "../types/channel";

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
      useServerStore.getState().removeServer(id);
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
      useVoicePresenceStore
        .getState()
        .addMember(channelId, { id: userId, name: userName });
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

    default:
      return;
  }
}

export function installWsBootstrap() {
  if (installed) return;
  installed = true;
  ws.on(dispatch);
}
