import { ws, WsEnvelope } from "./websocket";
import { useEntitlementsStore } from "../../features/subscription/store/entitlementsStore";
import { chatHandlers } from "../../features/chat/ws/chatHandlers";
import { dmHandlers } from "../../features/dm/ws/dmHandlers";
import { communityHandlers } from "../../features/communities/ws/communityHandlers";
import { socialHandlers } from "../../features/social/ws/socialHandlers";
import { voiceHandlers } from "../../features/voice/ws/voiceHandlers";
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
import { useCallStore } from "../../features/voice/store/callStore";
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
import { useHiddenDmStore } from "../../features/dm/store/hiddenDmStore";
import { installMessageCacheSync } from "../../features/chat/lib/messageCache";
import { installQuerySync } from "../query/queryClient";

export { setActiveChatView };

let installed = false;
let prefsReloadGen = 0;

const handlers: WsHandlers = {
  "conversation_pins.updated": () => {
    useConversationPinsStore.getState().remoteChanged();
  },
  ...communityHandlers,
  ...chatHandlers,
  ...dmHandlers,
  ...voiceHandlers,
  ...socialHandlers,
};

function dispatch(env: WsEnvelope) {
  const handler = handlers[env.type];
  if (!handler) return;
  handler(env.data as Record<string, unknown>, env);
}


export function installWsBootstrap() {
  if (installed) return;
  installed = true;
  installMessageCacheSync();
  installQuerySync();
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
    void useCallStore.getState().rehydrate();
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
