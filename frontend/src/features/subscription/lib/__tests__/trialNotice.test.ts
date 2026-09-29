import { dismissTrialNotice, trialEndsSoon, trialNoticeDismissed } from "../trialNotice";

const HOUR = 60 * 60 * 1000;

describe("trialEndsSoon", () => {
  const now = Date.parse("2026-10-05T12:00:00Z");

  it("shows the notice during the last day of a trial", () => {
    expect(trialEndsSoon(true, "2026-10-06T10:00:00Z", now)).toBe(true);
  });

  it("stays quiet earlier, after the end and for paid plans", () => {
    expect(trialEndsSoon(true, new Date(now + 30 * HOUR).toISOString(), now)).toBe(false);
    expect(trialEndsSoon(true, new Date(now - HOUR).toISOString(), now)).toBe(false);
    expect(trialEndsSoon(false, "2026-10-06T10:00:00Z", now)).toBe(false);
    expect(trialEndsSoon(true, null, now)).toBe(false);
  });
});

describe("trial notice dismissal", () => {
  it("remembers the dismissal per trial end", () => {
    expect(trialNoticeDismissed("2026-10-06T10:00:00Z")).toBe(false);
    dismissTrialNotice("2026-10-06T10:00:00Z");
    expect(trialNoticeDismissed("2026-10-06T10:00:00Z")).toBe(true);
    expect(trialNoticeDismissed("2026-11-06T10:00:00Z")).toBe(false);
  });
});
