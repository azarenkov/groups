import {
  settingsSyncedForCurrentUser,
  useNotificationSettingsStore,
} from "../store/notificationSettingsStore";

export function showDesktopNotification(title: string, body: string): void {
  try {
    if (typeof Notification === "undefined") return;
    if (Notification.permission !== "granted") return;
    if (document.visibilityState === "visible" && document.hasFocus()) return;
    const n = new Notification(title, { body, silent: true });
    n.onclick = () => {
      try {
        window.focus();
      } catch {}
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
  const onGesture = () => {
    if (!settingsSyncedForCurrentUser()) return;
    if (useNotificationSettingsStore.getState().desktopEnabled) {
      requestDesktopNotificationPermission();
    }
    window.removeEventListener("pointerdown", onGesture);
    window.removeEventListener("keydown", onGesture);
  };
  window.addEventListener("pointerdown", onGesture);
  window.addEventListener("keydown", onGesture);
}
