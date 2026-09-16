import type { WsHandlers } from "./types";
import type { Channel } from "../../types/channel";
import type { Server } from "../../types/server";
import { loadActivitiesFor, useActivityStore } from "../../store/activityStore";
import { useAuthStore } from "../../store/authStore";
import { useCallStore } from "../../store/callStore";
import { useChannelStore } from "../../store/channelStore";
import { useReadReceiptStore } from "../../store/readReceiptStore";
import { useServerStore } from "../../store/serverStore";
import { useUnreadStore } from "../../store/unreadStore";
import { useUserNamesStore } from "../../store/userNamesStore";
import { useVoicePresenceStore } from "../../store/voicePresenceStore";
import { asString, purgeServerLocally } from "./context";

export const serverHandlers: WsHandlers = {
  "server.created": (d) => {
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
  },
  "server.ownerChanged": (d) => {
      const id = asString(d.id);
      const ownerId = asString(d.ownerId);
      if (!id || !ownerId) return;
      useServerStore.getState().patchServer(id, { ownerId });
      return;
  },
  "server.updated": (d) => {
      const id = asString(d.id);
      if (!id) return;
      const patch: { name?: string; description?: string | null } = {};
      const name = asString(d.name);
      if (name) patch.name = name;
      patch.description = asString(d.description) ?? null;
      useServerStore.getState().patchServer(id, patch);
      return;
  },
  "server.deleted": (d) => {
      const id = asString(d.id);
      if (!id) return;
      purgeServerLocally(id);
      void loadActivitiesFor(Object.keys(useActivityStore.getState().byUser));
      return;
  },
  "server.memberLeft": (d) => {
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
      void import("../../store/rolesStore").then(({ useRolesStore }) => {
        const roles = useRolesStore.getState();
        roles.removeMember(serverId, userId);
        if (roles.byServer[serverId]) {
          void roles.fetch(serverId).catch(() => {});
        }
      });
      return;
  },
  "channel.created": (d) => {
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
        isDefault: d.isDefault === true,
      };
      useChannelStore.getState().addChannel(serverId, channel);
      return;
  },
  "channel.deleted": (d) => {
      const id = asString(d.id);
      const serverId = asString(d.serverId);
      if (!id || !serverId) return;
      useChannelStore.getState().removeChannel(serverId, id);
      useVoicePresenceStore.getState().clearChannel(id);
      useUnreadStore.getState().resetChannel(id);
      return;
  },
  "channel.renamed": (d) => {
      const id = asString(d.id);
      const serverId = asString(d.serverId);
      const newName = asString(d.newName);
      if (!id || !serverId || !newName) return;
      useChannelStore.getState().renameChannel(serverId, id, newName);
      return;
  },
  "server.memberJoined": (d) => {
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
  },
  "server.roles.changed": (d) => {
      const serverId = asString(d.serverId);
      if (!serverId) return;
      void import("../../store/rolesStore").then(({ useRolesStore }) => {
        const roles = useRolesStore.getState();
        if (roles.byServer[serverId] || roles.isTracked(serverId)) {
          void roles.fetch(serverId).catch(() => {});
        }
      });
      return;
  },
};
