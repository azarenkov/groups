import { appPath, type AppLocation } from "./appLocation";

type Navigate = (path: string) => void;

let navigate: Navigate | null = null;

export function registerAppNavigator(next: Navigate | null): void {
  navigate = next;
}

export function openAppLocation(location: AppLocation): void {
  navigate?.(appPath(location));
}
