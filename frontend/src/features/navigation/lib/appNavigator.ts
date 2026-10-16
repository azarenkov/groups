import { appPath, type AppLocation } from "./appLocation";

type Navigate = (path: string) => void;

let navigate: Navigate | null = null;
let currentEntry: string | null = null;

export function registerAppNavigator(next: Navigate | null): void {
  navigate = next;
}

export function noteAppEntry(entry: string): void {
  currentEntry = entry;
}

export function openAppLocation(location: AppLocation): void {
  navigate?.(appPath(location));
}

export async function openAppLocationOnceResolved(pending: Promise<AppLocation | null>): Promise<void> {
  const entry = currentEntry;
  const location = await pending;
  if (location && entry === currentEntry) openAppLocation(location);
}
