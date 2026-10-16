import {
  settingsSyncedForCurrentUser,
  useNotificationSettingsStore,
} from "../../features/settings/store/notificationSettingsStore";

export function showDesktopNotification(title: string, body: string, onOpen?: () => void): void {
  try {
    if (typeof Notification === "undefined") return;
    if (Notification.permission !== "granted") return;
    if (document.visibilityState === "visible" && document.hasFocus()) return;
    const n = new Notification(title, { body, silent: true });
    n.onclick = () => {
      try {
        window.focus();
      } catch {}
      onOpen?.();
      n.close();
    };
  } catch {}
}

export function requestDesktopNotificationPermission(): void {
  try {
    if (typeof Notification === "undefined") return;
    if (Notification.permission === "default") {
      void Notification.requestPermission();
    }
  } catch {}
}

export function initDesktopNotifications(): void {
  const disarm = () => {
    window.removeEventListener("pointerdown", onGesture);
    window.removeEventListener("keydown", onGesture);
  };
  const onGesture = () => {
    try {
      if (typeof Notification === "undefined" || Notification.permission !== "default") {
        disarm();
        return;
      }
    } catch {
      disarm();
      return;
    }
    if (!settingsSyncedForCurrentUser()) return;
    if (!useNotificationSettingsStore.getState().desktopEnabled) return;
    requestDesktopNotificationPermission();
    disarm();
  };
  window.addEventListener("pointerdown", onGesture);
  window.addEventListener("keydown", onGesture);
}
