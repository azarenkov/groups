import { appPath, type AppLocation } from "./appLocation";

type Navigate = (path: string) => void;

let navigate: Navigate | null = null;
let currentIntent: string | null = null;

export function registerAppNavigator(next: Navigate | null): void {
  navigate = next;
}

export function noteAppIntent(intent: string): void {
  currentIntent = intent;
}

export function openAppLocation(location: AppLocation): void {
  navigate?.(appPath(location));
}

export async function openAppLocationOnceResolved(pending: Promise<AppLocation | null>): Promise<void> {
  const intent = currentIntent;
  const location = await pending;
  if (location && intent === currentIntent) openAppLocation(location);
}
