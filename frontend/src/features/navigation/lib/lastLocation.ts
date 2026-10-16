const KEY = "rift.lastLocation.v1";

interface StoredLocation {
  userId: string;
  path: string;
}

export function rememberLocation(userId: string, path: string): void {
  try {
    localStorage.setItem(KEY, JSON.stringify({ userId, path } satisfies StoredLocation));
  } catch {}
}

export function lastLocationPath(userId: string): string | null {
  try {
    const stored = JSON.parse(localStorage.getItem(KEY) ?? "null") as StoredLocation | null;
    return stored && stored.userId === userId && typeof stored.path === "string" ? stored.path : null;
  } catch {
    return null;
  }
}

export function clearLastLocation(): void {
  try {
    localStorage.removeItem(KEY);
  } catch {}
}
