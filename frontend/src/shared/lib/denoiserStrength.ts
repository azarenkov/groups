export const FULL_ATTENUATION_DB = 100;
export const PARTIAL_ATTENUATION_MAX_DB = 40;

export function attenuationFor(strength: number): number {
  if (!Number.isFinite(strength)) return FULL_ATTENUATION_DB;
  const clamped = Math.min(1, Math.max(0, strength));
  if (clamped >= 1) return FULL_ATTENUATION_DB;
  return clamped * PARTIAL_ATTENUATION_MAX_DB;
}
