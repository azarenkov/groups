import {
  FULL_ATTENUATION_DB,
  MIN_ATTENUATION_DB,
  PARTIAL_ATTENUATION_MAX_DB,
  attenuationFor,
} from "../denoiserStrength";

describe("attenuationFor", () => {
  it("removes noise without a limit at full strength", () => {
    expect(attenuationFor(1)).toBe(FULL_ATTENUATION_DB);
    expect(attenuationFor(3)).toBe(FULL_ATTENUATION_DB);
  });

  it("barely touches the signal at zero strength but keeps the model running", () => {
    expect(attenuationFor(0)).toBe(MIN_ATTENUATION_DB);
    expect(attenuationFor(-1)).toBe(MIN_ATTENUATION_DB);
    expect(MIN_ATTENUATION_DB).toBeGreaterThanOrEqual(0.01);
  });

  it("scales the limit in between", () => {
    expect(attenuationFor(0.5)).toBeCloseTo(PARTIAL_ATTENUATION_MAX_DB / 2);
  });

  it("falls back to full strength on garbage", () => {
    expect(attenuationFor(Number.NaN)).toBe(FULL_ATTENUATION_DB);
  });
});
