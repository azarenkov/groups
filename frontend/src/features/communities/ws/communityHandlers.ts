import type { WsHandlers } from "../../../shared/ws/types";
import type { Channel } from "../../../shared/types/channel";
import type { Community } from "../../../shared/types/community";
import { useAuthStore } from "../../auth/store/authStore";
import { useCallStore } from "../../voice/store/callStore";
import { useChannelStore } from "../store/channelStore";
import { useReadReceiptStore } from "../../chat/store/readReceiptStore";
import { useCommunityStore } from "../store/communityStore";
import { useUnreadStore } from "../../chat/store/unreadStore";
import { useUserNamesStore } from "../../social/store/userNamesStore";
import { useVoicePresenceStore } from "../../voice/store/voicePresenceStore";
import { asString, purgeCommunityLocally } from "../../../shared/ws/context";
import { dropConversation } from "../../chat/lib/messageCache";

export const communityHandlers: WsHandlers = {
  "community.created": (d) => {
      const id = asString(d.id);
      const ownerId = asString(d.ownerId);
      const name = asString(d.name) ?? "";
      if (!id || !ownerId) return;
      const community: Community = {
        id,
        name,
        ownerId,
        description: (asString(d.description) ?? null) as string | null,
        accentColor: null,
        createdAt: asString(d.occurredAt) ?? new Date().toISOString(),
        updatedAt: asString(d.occurredAt) ?? new Date().toISOString(),
      };
      useCommunityStore.getState().addCommunity(community);
      return;
  },
  "community.ownerChanged": (d) => {
      const id = asString(d.id);
      const ownerId = asString(d.ownerId);
      if (!id || !ownerId) return;
      useCommunityStore.getState().patchCommunity(id, { ownerId });
      return;
  },
  "community.updated": (d) => {
      const id = asString(d.id);
      if (!id) return;
      const patch: { name?: string; description?: string | null } = {};
      const name = asString(d.name);
      if (name) patch.name = name;
      patch.description = asString(d.description) ?? null;
      useCommunityStore.getState().patchCommunity(id, patch);
      return;
  },
  "community.deleted": (d) => {
      const id = asString(d.id);
      if (!id) return;
      purgeCommunityLocally(id);
      return;
  },
  "community.memberLeft": (d) => {
      const communityId = asString(d.communityId);
      const userId = asString(d.userId);
      if (!communityId || !userId) return;
      const me = useAuthStore.getState().user;
      if (me && me.id === userId) {
        purgeCommunityLocally(communityId);
        return;
      }
      const channels = useChannelStore.getState().channelsByCommunity[communityId] ?? [];
      const receipts = useReadReceiptStore.getState();
      for (const c of channels) {
        receipts.removeChannelMember(c.id, userId);
      }
      useVoicePresenceStore
        .getState()
        .purgePendingMutes(channels.map((c) => c.id), userId, communityId);
      window.dispatchEvent(
        new CustomEvent("rift:community-members-changed", { detail: { communityId } }),
      );
      void import("../store/rolesStore").then(({ useRolesStore }) => {
        const roles = useRolesStore.getState();
        roles.removeMember(communityId, userId);
        if (roles.byCommunity[communityId]) {
          void roles.fetch(communityId).catch(() => {});
        }
      });
      return;
  },
  "channel.created": (d) => {
      const id = asString(d.id);
      const communityId = asString(d.communityId);
      const ownerId = asString(d.ownerId);
      const name = asString(d.name) ?? "";
      const kind = (d.kind === "voice" ? "voice" : "text") as Channel["kind"];
      const visibility = (d.visibility === "private"
        ? "private"
        : "public") as Channel["visibility"];
      if (!id || !communityId || !ownerId) return;
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
      useChannelStore.getState().addChannel(communityId, channel);
      return;
  },
  "channel.deleted": (d) => {
      const id = asString(d.id);
      const communityId = asString(d.communityId);
      if (!id || !communityId) return;
      useChannelStore.getState().removeChannel(communityId, id);
      dropConversation(id);
      useVoicePresenceStore.getState().clearChannel(id);
      useUnreadStore.getState().resetChannel(id);
      return;
  },
  "channel.renamed": (d) => {
      const id = asString(d.id);
      const communityId = asString(d.communityId);
      const newName = asString(d.newName);
      if (!id || !communityId || !newName) return;
      useChannelStore.getState().renameChannel(communityId, id, newName);
      return;
  },
  "community.memberJoined": (d) => {
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
      const joinedCommunityId = asString(d.communityId);
      if (joinedCommunityId) {
        window.dispatchEvent(
          new CustomEvent("rift:community-members-changed", {
            detail: { communityId: joinedCommunityId },
          }),
        );
      }
      return;
  },
  "community.roles.changed": (d) => {
      const communityId = asString(d.communityId);
      if (!communityId) return;
      void import("../store/rolesStore").then(({ useRolesStore }) => {
        const roles = useRolesStore.getState();
        if (roles.byCommunity[communityId] || roles.isTracked(communityId)) {
          void roles.fetch(communityId).catch(() => {});
        }
      });
      return;
  },
};
