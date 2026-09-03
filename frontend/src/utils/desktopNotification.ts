import { useNotificationSettingsStore } from "../store/notificationSettingsStore";

export function showDesktopNotification(title: string, body: string): void {
  try {
    if (typeof Notification === "undefined") return;
    if (Notification.permission !== "granted") return;
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
    if (useNotificationSettingsStore.getState().desktopEnabled) {
      requestDesktopNotificationPermission();
    }
  };
  window.addEventListener("pointerdown", onGesture, { once: true });
  window.addEventListener("keydown", onGesture, { once: true });
}
