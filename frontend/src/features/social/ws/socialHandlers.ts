import type { WsHandlers } from "../../../shared/ws/types";
import i18n from "../../../i18n";
import { showDesktopNotification } from "../../../shared/lib/desktopNotification";
import { desktopNotifAllowed } from "../../settings/store/notificationSettingsStore";
import { formatPlanDate } from "../../subscription/components/PlanUntil";
import { bustAvatar } from "../../../shared/lib/avatarCache";
import { parseBadgeStyle, parseMemberNumber, parseTier, parseValidUntil } from "../../subscription/lib/plans";
import { parseEmojiStatus } from "../../subscription/lib/emojiStatus";
import { useBadgeStore } from "../../subscription/store/badgeStore";
import { useEntitlementsStore } from "../../subscription/store/entitlementsStore";
import { loadActivitiesFor, loadFriendActivities, useActivityStore } from "../store/activityStore";
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

const TIER_PROJECTION_SETTLE_MS = [5000, 15000, 45000, 120000];

const bustAvatarAfterTierChange = (userId: string) => {
  bustAvatar("user", userId);
  for (const delay of TIER_PROJECTION_SETTLE_MS) {
    setTimeout(() => bustAvatar("user", userId), delay);
  }
};

export const socialHandlers: WsHandlers = {
  "friendship.requestSent": () => {
      void useFriendshipStore
        .getState()
        .fetchFriends()
        .then(() => loadFriendActivities());
      return;
  },
  "friendship.requestAccepted": (d) => {
      const selfId = useAuthStore.getState().user?.id;
      const accepterId = asString(d.accepterId);
      const senderId = asString(d.senderId);
      const peer = accepterId === selfId ? senderId : accepterId;
      if (peer && peer !== selfId) resyncPresence([peer]);
      void useFriendshipStore
        .getState()
        .fetchFriends()
        .then(() => loadFriendActivities());
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
      useActivityStore.getState().apply(peer, null);
      if (env.type === "user.unblocked") {
        void loadActivitiesFor([peer]);
        void import("../../chat/store/unreadStore").then(({ refetchUnread }) =>
          refetchUnread().catch(() => {}),
        );
      }
      scheduleHomeRefresh();
      void useFriendshipStore
        .getState()
        .fetchFriends()
        .then(() => loadFriendActivities());
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
      useActivityStore.getState().apply(peer, null);
      if (env.type === "user.unblocked") {
        void loadActivitiesFor([peer]);
        void import("../../chat/store/unreadStore").then(({ refetchUnread }) =>
          refetchUnread().catch(() => {}),
        );
      }
      scheduleHomeRefresh();
      void useFriendshipStore
        .getState()
        .fetchFriends()
        .then(() => loadFriendActivities());
      refetchDmPreviews();
      return;
  },
  "friendship.removed": (d) => {
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
  },
  "user.entitlementsChanged": (d) => {
      const userId = asString(d.userId);
      if (!userId) return;
      const tier = parseTier(d.tier);
      const memberNumber = parseMemberNumber(d.memberNumber);
      if (useAuthStore.getState().user?.id === userId) {
        useEntitlementsStore.getState().applyTier(tier, memberNumber, {
          validUntil: parseValidUntil(d.validUntil),
          trial: d.trial === true,
        });
        if (tier === "free") {
          bustAvatar("banner", userId);
        }
        bustAvatarAfterTierChange(userId);
        return;
      }
      useBadgeStore.getState().applyTier(userId, tier, memberNumber);
      useBadgeStore.getState().revalidateLater(userId);
      bustAvatar("banner", userId);
      bustAvatarAfterTierChange(userId);
  },
  "user.trialEnding": (d) => {
      const userId = asString(d.userId);
      const endsAt = parseValidUntil(d.endsAt);
      if (!userId || !endsAt || useAuthStore.getState().user?.id !== userId) return;
      if (Date.parse(endsAt) <= Date.now()) return;
      void useEntitlementsStore
        .getState()
        .fetch()
        .then((fresh) => {
          const { trial, validUntil } = useEntitlementsStore.getState();
          if (!fresh || !trial || validUntil !== endsAt || !desktopNotifAllowed()) return;
          const date = formatPlanDate(endsAt, i18n.language);
          showDesktopNotification(
            i18n.t("trial.endingSoonDesktopTitle", { ns: "subscription" }),
            i18n.t("trial.endingSoonDesktopBody", { ns: "subscription", date }),
          );
        });
  },
  "user.profileChanged": (d) => {
      const userId = asString(d.userId);
      if (!userId) return;
      const version = profileVersion(asString(d.occurredAt));
      const cached = useUserProfileStore.getState().bios[userId];
      if (cached && cached.at > version) return;
      if ("emojiStatus" in d) {
        const emojiStatus = parseEmojiStatus(d.emojiStatus);
        if (useAuthStore.getState().user?.id === userId) {
          useEntitlementsStore.getState().applyEmojiStatus(emojiStatus);
        } else {
          useBadgeStore.getState().applyEmojiStatus(userId, emojiStatus);
        }
      }
      if (typeof d.badgeStyle === "string") {
        const badgeStyle = parseBadgeStyle(d.badgeStyle);
        if (useAuthStore.getState().user?.id === userId) {
          useEntitlementsStore.getState().applyBadgeStyle(badgeStyle);
        } else {
          useBadgeStore.getState().applyStyle(userId, badgeStyle);
          useBadgeStore.getState().revalidateLater(userId);
        }
      }
      const bio = asString(d.bio) ?? "";
      const showActivity = d.showActivity === true;
      const showGameActivity = d.showGameActivity !== false;
      useUserProfileStore.getState().apply(userId, bio, version);
      const auth = useAuthStore.getState();
      if (!showGameActivity && auth.user?.id !== userId) {
        useActivityStore
          .getState()
          .applyEvent(userId, null, asString(d.occurredAt) ?? null);
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
        void useFriendshipStore
        .getState()
        .fetchFriends()
        .then(() => loadFriendActivities());
      }
      return;
  },
  "user.online": (d) => {
      const userId = asString(d.userId);
      if (userId && !isBlockedEitherWay(userId)) {
        usePresenceStore.getState().setOnline(userId);
        void loadActivitiesFor([userId]);
      }
      return;
  },
  "user.offline": (d) => {
      const userId = asString(d.userId);
      if (userId) {
        usePresenceStore.getState().setOffline(userId);
        useActivityStore.getState().apply(userId, null);
      }
      return;
  },
  "user.activityChanged": (d) => {
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
  },
};
