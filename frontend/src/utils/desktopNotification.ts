export function showDesktopNotification(title: string, body: string): void {
  try {
    if (typeof Notification === "undefined") return;
    if (document.hasFocus()) return;
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
