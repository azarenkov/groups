export function clearPersistentSessionData(): void {
  void import("../../features/communities/lib/sidebarSnapshot").then((m) => m.clearSidebarSnapshot());
  void import("./blobCache").then((m) => m.clearBlobCache());
}
