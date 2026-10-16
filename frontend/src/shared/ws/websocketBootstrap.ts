import { ws, WsEnvelope } from "./websocket";
import { useEntitlementsStore } from "../../features/subscription/store/entitlementsStore";
import { chatHandlers } from "../../features/chat/ws/chatHandlers";
import { dmHandlers } from "../../features/dm/ws/dmHandlers";
import { communityHandlers } from "../../features/communities/ws/communityHandlers";
import { socialHandlers } from "../../features/social/ws/socialHandlers";
import { dropHeldCalls, installStreamLimitNotice, voiceHandlers } from "../../features/voice/ws/voiceHandlers";
import type { WsHandlers } from "./types";
import {
  forgetReadThrottle,
  getActiveChannelView,
  getActiveDmView,
  scheduleUnreadRefetch,
  setActiveChatView,
  throttleMarkChannelRead,
  throttleMarkDmRead,
  viewIsForeground,
} from "./context";

import { acceptedFriendIds, useFriendshipStore } from "../../features/social/store/friendshipStore";
import { blockGeneration, useBlockStore } from "../../features/social/store/blockStore";
import { friendshipAPI } from "../../features/social/api/friendship";
import { peerOfDmPartition } from "../../features/chat/lib/partitions";
import { useDmPreviewStore } from "../../features/dm/store/dmPreviewStore";
import { installCallSessionSync, resumeCall } from "../../features/voice/store/callSessionSync";
import {
  loadActivitiesFor,
  loadFriendActivities,
  trackedActivityIds,
  useActivityStore,
} from "../../features/social/store/activityStore";
import { useAuthStore } from "../../features/auth/store/authStore";
import { useUserNamesStore } from "../../features/social/store/userNamesStore";
import { markNotificationPrefsStale, useNotificationPrefsStore } from "../../features/settings/store/notificationPrefsStore";
import { useNotificationSettingsStore } from "../../features/settings/store/notificationSettingsStore";
import { useConversationPinsStore } from "../../features/communities/store/conversationPinsStore";
import { refreshSearchHistory, revalidateSearchHistory } from "../../features/search/lib/searchHistory";
import { refreshInbox } from "../../features/inbox/lib/inbox";
import { useHiddenDmStore } from "../../features/dm/store/hiddenDmStore";
import { refetchDmPreviews } from "../../features/dm/store/dmPreviewStore";
import { admitDmEvent } from "../../features/dm/lib/dmCleared";
import { installMessageCacheSync } from "../../features/chat/lib/messageCache";
import { installQuerySync, queryClient } from "../query/queryClient";

export { setActiveChatView };

let installed = false;

function reconcilePairRelation(partition: string) {
  const selfId = useAuthStore.getState().user?.id;
  const peer = selfId ? peerOfDmPartition(partition, selfId) : null;
  if (!peer) return;
  const blocks = useBlockStore.getState();
  const known =
    !!useDmPreviewStore.getState().byFriend[peer] ||
    useFriendshipStore.getState().friends.some((f) => f.friendId === peer) ||
    !!blocks.blocked[peer] ||
    !!blocks.blockedBy[peer];
  if (!known) return;
  const gen = blockGeneration(peer);
  friendshipAPI
    .blockRelation(peer)
    .then((relation) => useBlockStore.getState().applyRelation(peer, relation, gen))
    .catch(() => {});
}
let prefsReloadGen = 0;

const handlers: WsHandlers = {
  "conversation_pins.updated": () => {
    useConversationPinsStore.getState().remoteChanged();
  },
  "search_history.updated": () => {
    refreshSearchHistory();
  },
  "notification.created": () => {
    refreshInbox();
  },
  "inbox.updated": () => {
    refreshInbox();
  },
  ...communityHandlers,
  ...chatHandlers,
  ...dmHandlers,
  ...voiceHandlers,
  ...socialHandlers,
};

const HISTORY_TARGET_EVENTS = new Set<string>([
  "message.edited",
  "dm_message.edited",
  "message.deleted",
  "message.hidden",
  "dm_message.deleted",
  "dm_conversation.cleared",
  "channel.deleted",
  "channel.renamed",
  "community.deleted",
  "community.updated",
  "user.renamed",
]);

export const touchesSearchHistory = (env: WsEnvelope): boolean => {
  if (HISTORY_TARGET_EVENTS.has(env.type)) return true;
  if (env.type !== "community.memberLeft") return false;
  const data = env.data as Record<string, unknown> | undefined;
  return data?.userId === useAuthStore.getState().user?.id;
};

function dispatch(env: WsEnvelope) {
  const handler = handlers[env.type];
  if (handler) handler(env.data as Record<string, unknown>, env);
  if (touchesSearchHistory(env)) revalidateSearchHistory();
}


export function installWsBootstrap() {
  if (installed) return;
  installed = true;
  installMessageCacheSync();
  installQuerySync();
  installStreamLimitNotice();
  installCallSessionSync();
  ws.addFilter((env) => admitDmEvent(env, useAuthStore.getState().user?.id));
  ws.onResync((partition) => {
    reconcilePairRelation(partition);
    refetchDmPreviews(true);
    scheduleUnreadRefetch();
    void queryClient.invalidateQueries({ queryKey: ["pins"] });
    void useFriendshipStore.getState().fetchFriends().catch(() => {});
    void useBlockStore.getState().fetch().catch(() => {});
    useHiddenDmStore.getState().resync();
    revalidateSearchHistory();
    refreshInbox();
  });
  const onForeground = () => {
    if (!viewIsForeground()) return;
    const channelId = getActiveChannelView();
    const friendId = getActiveDmView();
    forgetReadThrottle({ channelId: channelId ?? undefined, friendId: friendId ?? undefined });
    if (channelId) throttleMarkChannelRead(channelId);
    if (friendId) throttleMarkDmRead(friendId);
    if (channelId || friendId) scheduleUnreadRefetch();
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
      void useEntitlementsStore.getState().fetch();
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
    void import("../../features/communities/store/rolesStore").then(({ useRolesStore }) => {
      const roles = useRolesStore.getState();
      const tracked = new Set([
        ...Object.keys(roles.byCommunity),
        ...roles.trackedCommunities(),
      ]);
      for (const communityId of tracked) {
        void roles.fetch(communityId).catch(() => {});
      }
    });
    void useConversationPinsStore
      .getState()
      .fetch()
      .catch((err) => console.warn("[ws] failed to reload conversation pins:", err));
    useHiddenDmStore.getState().markStale();
    revalidateSearchHistory();
    refreshInbox();
    dropHeldCalls();
    void resumeCall();
    useUserNamesStore.getState().bumpSyncTick();
    void loadActivitiesFor(trackedActivityIds());
    const friendsBefore = acceptedFriendIds();
    void useFriendshipStore
      .getState()
      .fetchFriends()
      .then(() => {
        if (useFriendshipStore.getState().error === null) {
          useUserNamesStore.getState().clear();
          const after = new Set(acceptedFriendIds());
          const gone = friendsBefore.filter((id) => !after.has(id));
          for (const id of gone) useActivityStore.getState().apply(id, null);
          if (gone.length > 0) void loadActivitiesFor(gone);
          void loadFriendActivities();
        }
      });
  });
}
