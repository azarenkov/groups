import { makeupGainFor } from "../makeupGain";

describe("makeupGainFor", () => {
  it("leaves the gain alone while nobody speaks", () => {
    expect(makeupGainFor(0.001, 0.0002, 1)).toBe(1);
  });

  it("leaves the gain alone when the denoiser is silent", () => {
    expect(makeupGainFor(0.2, 0, 1.5)).toBe(1.5);
  });

  it("moves towards the attenuation the denoiser introduced", () => {
    let gain = 1;
    for (let i = 0; i < 60; i++) gain = makeupGainFor(0.2, 0.05, gain);
    expect(gain).toBeCloseTo(4, 1);
  });

  it("never attenuates and never explodes", () => {
    let quiet = 1;
    for (let i = 0; i < 60; i++) quiet = makeupGainFor(0.05, 0.2, quiet);
    expect(quiet).toBeCloseTo(1, 2);

    let loud = 1;
    for (let i = 0; i < 200; i++) loud = makeupGainFor(0.5, 0.001, loud);
    expect(loud).toBeLessThanOrEqual(6);
  });

  it("approaches the target gradually rather than in one step", () => {
    const first = makeupGainFor(0.2, 0.05, 1);
    expect(first).toBeGreaterThan(1);
    expect(first).toBeLessThan(2);
  });
});
