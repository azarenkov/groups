import { isSpeech, makeupGainFor } from "../makeupGain";

describe("isSpeech", () => {
  it("rejects steady noise the denoiser removes", () => {
    expect(isSpeech(0.02, 0.004, true)).toBe(false);
  });

  it("rejects noise that starts suddenly in a quiet room", () => {
    let gain = 1;
    for (let i = 0; i < 400; i++) {
      gain = makeupGainFor(0.02, 0.004, gain, isSpeech(0.02, 0.004, true));
    }
    expect(gain).toBeCloseTo(1, 2);
  });

  it("accepts speech the denoiser mostly keeps", () => {
    expect(isSpeech(0.2, 0.15, true)).toBe(true);
  });

  it("rejects everything while the gate is closed", () => {
    expect(isSpeech(0.5, 0.4, false)).toBe(false);
  });

  it("rejects input below the absolute floor", () => {
    expect(isSpeech(0.005, 0.004, true)).toBe(false);
  });

  it("rejects a silent denoiser output", () => {
    expect(isSpeech(0.2, 0, true)).toBe(false);
  });
});

describe("makeupGainFor", () => {
  it("stays at unity while nobody speaks", () => {
    expect(makeupGainFor(0.001, 0.0002, 1, false)).toBe(1);
  });

  it("releases a raised gain back to unity during silence", () => {
    let gain = 4;
    const firstStep = makeupGainFor(0.001, 0.0002, gain, false);
    expect(firstStep).toBeLessThan(gain);
    expect(firstStep).toBeGreaterThan(1);
    for (let i = 0; i < 400; i++) {
      gain = makeupGainFor(0.001, 0.0002, gain, false);
    }
    expect(gain).toBeCloseTo(1, 2);
  });

  it("releases rather than freezing when the denoiser goes silent", () => {
    expect(makeupGainFor(0.2, 0, 1.5, true)).toBeLessThan(1.5);
  });

  it("moves towards the attenuation the denoiser introduced", () => {
    let gain = 1;
    for (let i = 0; i < 60; i++) gain = makeupGainFor(0.2, 0.1, gain, true);
    expect(gain).toBeCloseTo(2, 1);
  });

  it("never attenuates and never explodes", () => {
    let quiet = 1;
    for (let i = 0; i < 60; i++) quiet = makeupGainFor(0.05, 0.2, quiet, true);
    expect(quiet).toBeCloseTo(1, 2);

    let loud = 1;
    for (let i = 0; i < 200; i++) loud = makeupGainFor(0.5, 0.001, loud, true);
    expect(loud).toBeLessThanOrEqual(6);
  });

  it("cannot be driven past the ratio that still counts as speech", () => {
    let gain = 1;
    for (let i = 0; i < 400; i++) {
      const input = 0.2;
      const denoised = 0.07;
      gain = makeupGainFor(input, denoised, gain, isSpeech(input, denoised, true));
    }
    expect(gain).toBeLessThanOrEqual(1 / 0.35 + 0.01);
  });

  it("approaches the target gradually rather than in one step", () => {
    const first = makeupGainFor(0.2, 0.1, 1, true);
    expect(first).toBeGreaterThan(1);
    expect(first).toBeLessThan(2);
  });
});
