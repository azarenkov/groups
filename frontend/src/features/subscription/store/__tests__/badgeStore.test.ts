import { entitlementsAPI } from "../../api/entitlements";
import { useBadgeStore } from "../badgeStore";

jest.mock("../../api/entitlements", () => ({
  entitlementsAPI: {
    getMine: jest.fn(),
    setBadgeStyle: jest.fn(),
    lookupBadges: jest.fn(),
  },
}));

const mockedApi = entitlementsAPI as jest.Mocked<typeof entitlementsAPI>;

describe("badgeStore tier events", () => {
  beforeEach(() => {
    jest.useFakeTimers();
    useBadgeStore.getState().clear();
    mockedApi.lookupBadges.mockResolvedValue([]);
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("drops the badge on downgrade without waiting for a lookup", () => {
    useBadgeStore.getState().apply("u1", { tier: "pro", badgeStyle: "ink", memberNumber: 7 });
    useBadgeStore.getState().applyTier("u1", "free", null);
    expect(useBadgeStore.getState().byUser.u1).toBeNull();
    expect(mockedApi.lookupBadges).not.toHaveBeenCalled();
  });

  it("keeps the chosen style when only the tier changes", () => {
    useBadgeStore.getState().apply("u1", { tier: "pro", badgeStyle: "ink", memberNumber: 7 });
    useBadgeStore.getState().applyTier("u1", "premium", null);
    expect(useBadgeStore.getState().byUser.u1).toEqual({ tier: "premium", badgeStyle: "ink", memberNumber: 7 });
  });

  it("revalidates after the projection had time to catch up", async () => {
    mockedApi.lookupBadges.mockResolvedValue([{ userId: "u1", tier: "premium", badgeStyle: "solid", memberNumber: 7 }]);
    useBadgeStore.getState().applyTier("u1", "premium", null);
    useBadgeStore.getState().revalidateLater("u1");
    jest.advanceTimersByTime(1_000);
    expect(mockedApi.lookupBadges).not.toHaveBeenCalled();
    jest.advanceTimersByTime(5_000);
    expect(mockedApi.lookupBadges).toHaveBeenCalledWith(["u1"]);
    await Promise.resolve();
    await Promise.resolve();
    expect(useBadgeStore.getState().byUser.u1).toEqual({ tier: "premium", badgeStyle: "solid", memberNumber: 7 });
  });
});

describe("badgeStore lookup retries", () => {
  beforeEach(() => {
    jest.useFakeTimers();
    useBadgeStore.getState().clear();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  const settle = async () => {
    for (let i = 0; i < 4; i += 1) await Promise.resolve();
  };

  it("queues a failed batch again with backoff", async () => {
    mockedApi.lookupBadges
      .mockRejectedValueOnce(new Error("down"))
      .mockResolvedValueOnce([{ userId: "u1", tier: "pro", badgeStyle: "tint", memberNumber: 3 }]);
    useBadgeStore.getState().request("u1");
    jest.advanceTimersByTime(50);
    await settle();
    expect(mockedApi.lookupBadges).toHaveBeenCalledTimes(1);
    jest.advanceTimersByTime(500);
    expect(mockedApi.lookupBadges).toHaveBeenCalledTimes(1);
    jest.advanceTimersByTime(600);
    await settle();
    expect(mockedApi.lookupBadges).toHaveBeenCalledTimes(2);
    expect(useBadgeStore.getState().byUser.u1).toEqual({ tier: "pro", badgeStyle: "tint", memberNumber: 3 });
  });

  it("gives up after the retry budget", async () => {
    mockedApi.lookupBadges.mockRejectedValue(new Error("down"));
    useBadgeStore.getState().request("u1");
    for (let i = 0; i < 6; i += 1) {
      jest.advanceTimersByTime(10_000);
      await settle();
    }
    expect(mockedApi.lookupBadges).toHaveBeenCalledTimes(4);
  });
});
