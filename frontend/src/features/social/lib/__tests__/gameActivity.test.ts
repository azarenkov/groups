import { activityAllowed } from "../gameActivity";

describe("publishing what you are playing", () => {
  it("is allowed by default", () => {
    expect(
      activityAllowed({ hiddenForUser: false, appearOffline: false }),
    ).toBe(true);
  });

  it("stops when the privacy switch is off", () => {
    expect(
      activityAllowed({
        showGameActivity: false,
        hiddenForUser: false,
        appearOffline: false,
      }),
    ).toBe(false);
  });

  it("stops while you appear offline", () => {
    expect(
      activityAllowed({
        showGameActivity: true,
        hiddenForUser: false,
        appearOffline: true,
      }),
    ).toBe(false);
  });

  it("resumes once you are visible again", () => {
    expect(
      activityAllowed({
        showGameActivity: true,
        hiddenForUser: false,
        appearOffline: false,
      }),
    ).toBe(true);
  });

  it("stays off for a user the server told us to hide", () => {
    expect(
      activityAllowed({ hiddenForUser: true, appearOffline: false }),
    ).toBe(false);
  });
});
