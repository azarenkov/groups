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
    useBadgeStore.getState().apply("u1", { tier: "pro", badgeStyle: "ink" });
    useBadgeStore.getState().applyTier("u1", "free");
    expect(useBadgeStore.getState().byUser.u1).toBeNull();
    expect(mockedApi.lookupBadges).not.toHaveBeenCalled();
  });

  it("keeps the chosen style when only the tier changes", () => {
    useBadgeStore.getState().apply("u1", { tier: "pro", badgeStyle: "ink" });
    useBadgeStore.getState().applyTier("u1", "premium");
    expect(useBadgeStore.getState().byUser.u1).toEqual({ tier: "premium", badgeStyle: "ink" });
  });

  it("revalidates after the projection had time to catch up", async () => {
    mockedApi.lookupBadges.mockResolvedValue([{ userId: "u1", tier: "premium", badgeStyle: "solid" }]);
    useBadgeStore.getState().applyTier("u1", "premium");
    useBadgeStore.getState().revalidateLater("u1");
    jest.advanceTimersByTime(1_000);
    expect(mockedApi.lookupBadges).not.toHaveBeenCalled();
    jest.advanceTimersByTime(5_000);
    expect(mockedApi.lookupBadges).toHaveBeenCalledWith(["u1"]);
    await Promise.resolve();
    await Promise.resolve();
    expect(useBadgeStore.getState().byUser.u1).toEqual({ tier: "premium", badgeStyle: "solid" });
  });
});
