export function resetSessionStores(outgoingAccessToken?: string | null): void {
  void import("../services/gameActivity").then((m) =>
    m.clearActivityForLogout(outgoingAccessToken ?? null),
  );
  void import("../utils/avatarCache").then((m) => m.clearAvatarCache());

  void import("./activityStore").then((m) => m.useActivityStore.getState().clearAll());
  void import("./blockStore").then((m) => m.useBlockStore.getState().clearAll());
  void import("./dmPreviewStore").then((m) => m.useDmPreviewStore.getState().clear());
  void import("./friendshipStore").then((m) => m.useFriendshipStore.getState().clearAll());
  void import("./homeStore").then((m) => m.useHomeStore.getState().clear());
  void import("./notificationPrefsStore").then((m) =>
    m.useNotificationPrefsStore.getState().clearAll(),
  );
  void import("./presenceStore").then((m) => m.usePresenceStore.getState().clearAll());
  void import("./readReceiptStore").then((m) => m.useReadReceiptStore.getState().clear());
  void import("./rolesStore").then((m) => m.useRolesStore.getState().clearAll());
  void import("./streamViewersStore").then((m) =>
    m.useStreamViewersStore.getState().clearAll(),
  );
  void import("./unreadStore").then((m) => m.useUnreadStore.getState().clearAll());
  void import("./userNamesStore").then((m) => m.useUserNamesStore.getState().clear());
  void import("./userProfileStore").then((m) => m.useUserProfileStore.getState().clear());
  void import("./voicePresenceStore").then((m) =>
    m.useVoicePresenceStore.getState().clearAll(),
  );

  void import("./voiceSettingsStore").then((m) => m.invalidateVoiceSettingsSync());
  void import("./notificationSettingsStore").then((m) =>
    m.invalidateNotificationSettingsSync(),
  );
  void import("./hotkeySettingsStore").then((m) => m.invalidateHotkeySettingsSync());
}
