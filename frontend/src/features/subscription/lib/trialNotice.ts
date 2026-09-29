export const TRIAL_NOTICE_LEAD_MS = 24 * 60 * 60 * 1000;

const storageKey = (validUntil: string) => `rift.trialNotice.${validUntil}`;

export const trialEndsSoon = (trial: boolean, validUntil: string | null, now: number): boolean => {
  if (!trial || !validUntil) return false;
  const left = Date.parse(validUntil) - now;
  return left > 0 && left <= TRIAL_NOTICE_LEAD_MS;
};

export const trialNoticeDismissed = (validUntil: string): boolean => {
  try {
    return localStorage.getItem(storageKey(validUntil)) === "1";
  } catch {
    return false;
  }
};

export const dismissTrialNotice = (validUntil: string): void => {
  try {
    localStorage.setItem(storageKey(validUntil), "1");
  } catch {}
};
