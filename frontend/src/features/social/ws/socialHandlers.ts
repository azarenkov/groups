import type { WsHandlers } from "../../../shared/ws/types";
import { parseEmojiStatus } from "../lib/emojiStatus";
import { useEmojiStatusStore } from "../store/emojiStatusStore";
import { useAuthStore } from "../../auth/store/authStore";
import { isBlockedEitherWay, resyncPresence, useBlockStore } from "../store/blockStore";
import { useCallStore } from "../../voice/store/callStore";
import { refetchDmPreviews } from "../../dm/store/dmPreviewStore";
import { useFriendshipStore } from "../store/friendshipStore";
import { scheduleHomeRefresh, useHomeStore } from "../../home/store/homeStore";
import { usePresenceStore } from "../store/presenceStore";
import { useUnreadStore } from "../../chat/store/unreadStore";
import { useUserNamesStore } from "../store/userNamesStore";
import { profileVersion, useUserProfileStore } from "../store/userProfileStore";
import { useVoicePresenceStore } from "../../voice/store/voicePresenceStore";
import { renameMentionTarget } from "../../chat/lib/mentionTargets";
import { asString } from "../../../shared/ws/context";

export const socialHandlers: WsHandlers = {
  "friendship.requestSent": () => {
      void useFriendshipStore.getState().fetchFriends();
      return;
  },
  "friendship.requestAccepted": (d) => {
      const selfId = useAuthStore.getState().user?.id;
      const accepterId = asString(d.accepterId);
      const senderId = asString(d.senderId);
      const peer = accepterId === selfId ? senderId : accepterId;
      if (peer && peer !== selfId) resyncPresence([peer]);
      void useFriendshipStore.getState().fetchFriends();
      return;
  },
  "user.blocked": (d, env) => {
      const blockerId = asString(d.blockerId);
      const blockedId = asString(d.blockedId);
      const selfId = useAuthStore.getState().user?.id;
      if (!blockerId || !blockedId || !selfId) return;
      const applied = useBlockStore
        .getState()
        .applyEvent(
          selfId,
          blockerId,
          blockedId,
          env.type === "user.blocked",
          asString(d.occurredAt),
        );
      if (!applied) return;
      const peer = blockerId === selfId ? blockedId : blockerId;
      if (env.type === "user.blocked") {
        useUnreadStore.getState().resetDm(peer);
        void import("../../chat/store/unreadStore").then(({ refetchUnread }) =>
          refetchUnread().catch(() => {}),
        );
        usePresenceStore.getState().setOffline(peer);
        const calls = useCallStore.getState();
        if (calls.incoming?.callerId === peer) calls.clearIncoming();
        calls.clearJoinable(peer);
        const sharesCall =
          !!calls.activeCallId &&
          (!!calls.participants[peer] || calls.ringing.includes(peer) || calls.peerId === peer);
        if (blockerId === selfId && sharesCall) void calls.leave().catch(() => {});
      }
      if (env.type === "user.unblocked") {
        void import("../../chat/store/unreadStore").then(({ refetchUnread }) =>
          refetchUnread().catch(() => {}),
        );
      }
      scheduleHomeRefresh();
      void useFriendshipStore.getState().fetchFriends();
      refetchDmPreviews();
      return;
  },
  "user.unblocked": (d, env) => {
      const blockerId = asString(d.blockerId);
      const blockedId = asString(d.blockedId);
      const selfId = useAuthStore.getState().user?.id;
      if (!blockerId || !blockedId || !selfId) return;
      const applied = useBlockStore
        .getState()
        .applyEvent(
          selfId,
          blockerId,
          blockedId,
          env.type === "user.blocked",
          asString(d.occurredAt),
        );
      if (!applied) return;
      const peer = blockerId === selfId ? blockedId : blockerId;
      if (env.type === "user.blocked") {
        useUnreadStore.getState().resetDm(peer);
        void import("../../chat/store/unreadStore").then(({ refetchUnread }) =>
          refetchUnread().catch(() => {}),
        );
        usePresenceStore.getState().setOffline(peer);
        const calls = useCallStore.getState();
        if (calls.incoming?.callerId === peer) calls.clearIncoming();
        calls.clearJoinable(peer);
        const sharesCall =
          !!calls.activeCallId &&
          (!!calls.participants[peer] || calls.ringing.includes(peer) || calls.peerId === peer);
        if (blockerId === selfId && sharesCall) void calls.leave().catch(() => {});
      }
      if (env.type === "user.unblocked") {
        void import("../../chat/store/unreadStore").then(({ refetchUnread }) =>
          refetchUnread().catch(() => {}),
        );
      }
      scheduleHomeRefresh();
      void useFriendshipStore.getState().fetchFriends();
      refetchDmPreviews();
      return;
  },
  "friendship.removed": () => {
      void useFriendshipStore.getState().fetchFriends();
      refetchDmPreviews();
      return;
  },
  "user.profileChanged": (d) => {
      const userId = asString(d.userId);
      if (!userId) return;
      const version = profileVersion(asString(d.occurredAt));
      if ("emojiStatus" in d) {
        useEmojiStatusStore.getState().apply(userId, parseEmojiStatus(d.emojiStatus), version);
      }
      const cached = useUserProfileStore.getState().bios[userId];
      if (cached && cached.at > version) return;
      const bio = asString(d.bio) ?? "";
      const showActivity = d.showActivity === true;
      useUserProfileStore.getState().apply(userId, bio, version);
      const auth = useAuthStore.getState();
      if (auth.user?.id === userId) {
        auth.setUser({ ...auth.user, bio, showActivity });
      } else if (!showActivity) {
        useHomeStore.getState().hideFriendActivity(userId, version);
        scheduleHomeRefresh();
      } else {
        useHomeStore.getState().unhideFriendActivity(userId, version);
        scheduleHomeRefresh();
      }
      return;
  },
  "user.renamed": (d) => {
      const userId = asString(d.userId);
      const newName = asString(d.name);
      if (!userId) return;
      renameMentionTarget(userId, newName);
      if (newName) {
        const occurredAt = asString(d.occurredAt);
        const applied = useUserNamesStore
          .getState()
          .applyRename(userId, newName, occurredAt);
        if (!applied) return;
        useVoicePresenceStore.getState().renameMember(userId, newName);
        useCallStore.getState().participantRenamed(userId, newName);
        useFriendshipStore.getState().applyName(userId, newName);
      }
      const me = useAuthStore.getState().user;
      if (me && me.id === userId) {
        void useAuthStore.getState().fetchMe();
      } else {
        void useFriendshipStore.getState().fetchFriends();
      }
      return;
  },
  "user.online": (d) => {
      const userId = asString(d.userId);
      if (userId && !isBlockedEitherWay(userId)) usePresenceStore.getState().setOnline(userId);
      return;
  },
  "user.offline": (d) => {
      const userId = asString(d.userId);
      if (userId) usePresenceStore.getState().setOffline(userId);
      return;
  },
};
