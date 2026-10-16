export function clearPersistentSessionData(): void {
  void import("../../features/communities/lib/sidebarSnapshot").then((m) => m.clearSidebarSnapshot());
  void import("./blobCache").then((m) => m.clearBlobCache());
  void import("../../features/search/lib/searchHistory").then((m) => m.clearStoredHistory());
  void import("../../features/navigation/lib/lastLocation").then((m) => m.clearLastLocation());
}
