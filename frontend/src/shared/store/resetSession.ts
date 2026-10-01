export function resetSessionStores(outgoingAccessToken?: string | null): void {
  void import("../../features/social/lib/gameActivity").then((m) =>
    m.clearActivityForLogout(outgoingAccessToken ?? null),
  );
  void import("../lib/avatarCache").then((m) => m.clearAvatarCache());
  void import("../../features/chat/lib/mentionTargets").then((m) => m.clearMentionTargets());

  void import("../../features/social/store/activityStore").then((m) => m.useActivityStore.getState().clearAll());
  void import("../../features/social/store/blockStore").then((m) => m.useBlockStore.getState().clearAll());
  void import("../../features/dm/store/dmPreviewStore").then((m) => m.useDmPreviewStore.getState().clear());
  void import("../../features/social/store/friendshipStore").then((m) => m.useFriendshipStore.getState().clearAll());
  void import("../../features/home/store/homeStore").then((m) => m.useHomeStore.getState().clear());
  void import("../../features/settings/store/notificationPrefsStore").then((m) =>
    m.useNotificationPrefsStore.getState().clearAll(),
  );
  void import("../../features/social/store/presenceStore").then((m) => m.usePresenceStore.getState().clearAll());
  void import("../../features/chat/store/readReceiptStore").then((m) => m.useReadReceiptStore.getState().clear());
  void import("../../features/communities/store/rolesStore").then((m) => m.useRolesStore.getState().clearAll());
  void import("../../features/voice/store/streamViewersStore").then((m) =>
    m.useStreamViewersStore.getState().clearAll(),
  );
  void import("../../features/chat/store/unreadStore").then((m) => m.useUnreadStore.getState().clearAll());
  void import("../../features/subscription/store/entitlementsStore").then((m) => m.useEntitlementsStore.getState().clear());
  void import("../../features/stickers/store/stickerStore").then((m) => m.useStickerStore.getState().clear());
  void import("../../features/subscription/store/badgeStore").then((m) => m.useBadgeStore.getState().clear());
  void import("../../features/social/store/userNamesStore").then((m) => m.useUserNamesStore.getState().clear());
  void import("../../features/social/store/userProfileStore").then((m) => m.useUserProfileStore.getState().clear());
  void import("../../features/voice/store/voicePresenceStore").then((m) =>
    m.useVoicePresenceStore.getState().clearAll(),
  );
  void import("../../features/voice/store/voiceSessionStore").then((m) =>
    m.useVoiceSessionStore.getState().clearAll(),
  );

  void import("../../features/voice/store/voiceSettingsStore").then((m) => m.invalidateVoiceSettingsSync());
  void import("../../features/settings/store/notificationSettingsStore").then((m) =>
    m.invalidateNotificationSettingsSync(),
  );
  void import("../../features/settings/store/hotkeySettingsStore").then((m) => m.invalidateHotkeySettingsSync());
}
