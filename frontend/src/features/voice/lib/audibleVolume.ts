import { getLiveKitService } from "./livekit";

const lastAudible = new Map<string, number>();
let tracking = false;

export function rememberAudibleVolume(userId: string, volume: number): void {
  if (volume > 0) lastAudible.set(userId, volume);
}

export function audibleVolume(userId: string): number {
  return lastAudible.get(userId) ?? 1;
}

export function trackAudibleVolumes(): void {
  if (tracking) return;
  tracking = true;
  getLiveKitService().subscribeRemoteVolume((userId, volume) => rememberAudibleVolume(userId, volume));
}
