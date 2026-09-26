const lastAudible = new Map<string, number>();

export function rememberAudibleVolume(userId: string, volume: number): void {
  if (volume > 0) lastAudible.set(userId, volume);
}

export function audibleVolume(userId: string): number {
  return lastAudible.get(userId) ?? 1;
}
