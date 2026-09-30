import { ApiError } from "../../../../shared/lib/apiError";
import { useEntitlementsStore } from "../../store/entitlementsStore";
import { whilePlanSyncs } from "../planSync";

jest.mock("../../api/entitlements", () => ({
  entitlementsAPI: { getMine: jest.fn(), startTrial: jest.fn(), setBadgeStyle: jest.fn(), lookupBadges: jest.fn() },
}));

const proRequired = () => new ApiError("pro", 403, "PRO_REQUIRED");

const flush = async () => {
  for (let i = 0; i < 10; i += 1) await Promise.resolve();
};

describe("whilePlanSyncs", () => {
  beforeEach(() => {
    jest.useFakeTimers();
    useEntitlementsStore.setState({ tier: "pro" });
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it("retries until media sees the upgraded plan", async () => {
    const action = jest.fn().mockRejectedValueOnce(proRequired()).mockResolvedValueOnce("ok");
    const result = whilePlanSyncs(action);
    await flush();
    jest.advanceTimersByTime(2000);
    await flush();
    await expect(result).resolves.toBe("ok");
    expect(action).toHaveBeenCalledTimes(2);
  });

  it("gives up after the last delay", async () => {
    const action = jest.fn().mockRejectedValue(proRequired());
    const result = whilePlanSyncs(action);
    const settled = result.catch((err) => err);
    for (const delay of [2000, 5000, 10000]) {
      await flush();
      jest.advanceTimersByTime(delay);
    }
    await flush();
    expect(await settled).toBeInstanceOf(ApiError);
    expect(action).toHaveBeenCalledTimes(4);
  });

  it("does not retry for a free account or other errors", async () => {
    useEntitlementsStore.setState({ tier: "free" });
    await expect(whilePlanSyncs(jest.fn().mockRejectedValue(proRequired()))).rejects.toBeInstanceOf(ApiError);
    useEntitlementsStore.setState({ tier: "pro" });
    const other = new ApiError("bad", 400, "IMAGE_CONTENT_REJECTED");
    const action = jest.fn().mockRejectedValue(other);
    await expect(whilePlanSyncs(action)).rejects.toBe(other);
    expect(action).toHaveBeenCalledTimes(1);
  });
});
