import {
  FULL_ATTENUATION_DB,
  PARTIAL_ATTENUATION_MAX_DB,
  attenuationFor,
} from "../denoiserStrength";

describe("attenuationFor", () => {
  it("removes noise without a limit at full strength", () => {
    expect(attenuationFor(1)).toBe(FULL_ATTENUATION_DB);
    expect(attenuationFor(3)).toBe(FULL_ATTENUATION_DB);
  });

  it("leaves the signal alone at zero strength", () => {
    expect(attenuationFor(0)).toBe(0);
    expect(attenuationFor(-1)).toBe(0);
  });

  it("scales the limit in between", () => {
    expect(attenuationFor(0.5)).toBeCloseTo(PARTIAL_ATTENUATION_MAX_DB / 2);
  });

  it("falls back to full strength on garbage", () => {
    expect(attenuationFor(Number.NaN)).toBe(FULL_ATTENUATION_DB);
  });
});
