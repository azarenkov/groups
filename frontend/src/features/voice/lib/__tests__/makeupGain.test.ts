import {
  MAKEUP_SPEECH_FLOOR,
  isSpeech,
  makeupGainFor,
  updateNoiseFloor,
} from "../makeupGain";

const settle = (rms: number, steps = 200) => {
  let floor = MAKEUP_SPEECH_FLOOR;
  for (let i = 0; i < steps; i++) floor = updateNoiseFloor(floor, rms);
  return floor;
};

describe("noise floor", () => {
  it("follows a quiet room down", () => {
    expect(settle(0.002)).toBeCloseTo(0.002, 3);
  });

  it("rises to steady background noise", () => {
    expect(settle(0.02)).toBeCloseTo(0.02, 3);
  });
});

describe("isSpeech", () => {
  it("rejects steady noise the gate lets through", () => {
    const floor = settle(0.02);
    expect(isSpeech(0.02, floor, true)).toBe(false);
  });

  it("accepts speech well above the noise floor", () => {
    const floor = settle(0.02);
    expect(isSpeech(0.2, floor, true)).toBe(true);
  });

  it("rejects everything while the gate is closed", () => {
    expect(isSpeech(0.5, MAKEUP_SPEECH_FLOOR, false)).toBe(false);
  });

  it("rejects input below the absolute floor", () => {
    expect(isSpeech(0.005, 0.0001, true)).toBe(false);
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
    for (let i = 0; i < 400; i++) gain = makeupGainFor(0.001, 0.0002, gain, false);
    expect(gain).toBeCloseTo(1, 2);
  });

  it("does not amplify steady background noise", () => {
    const floor = settle(0.02);
    let gain = 1;
    for (let i = 0; i < 200; i++) {
      const speech = isSpeech(0.02, floor, true);
      gain = makeupGainFor(0.02, 0.004, gain, speech);
    }
    expect(gain).toBeCloseTo(1, 2);
  });

  it("releases rather than freezing when the denoiser goes silent", () => {
    expect(makeupGainFor(0.2, 0, 1.5, true)).toBeLessThan(1.5);
  });

  it("moves towards the attenuation the denoiser introduced", () => {
    let gain = 1;
    for (let i = 0; i < 60; i++) gain = makeupGainFor(0.2, 0.05, gain, true);
    expect(gain).toBeCloseTo(4, 1);
  });

  it("never attenuates and never explodes", () => {
    let quiet = 1;
    for (let i = 0; i < 60; i++) quiet = makeupGainFor(0.05, 0.2, quiet, true);
    expect(quiet).toBeCloseTo(1, 2);

    let loud = 1;
    for (let i = 0; i < 200; i++) loud = makeupGainFor(0.5, 0.001, loud, true);
    expect(loud).toBeLessThanOrEqual(6);
  });

  it("approaches the target gradually rather than in one step", () => {
    const first = makeupGainFor(0.2, 0.05, 1, true);
    expect(first).toBeGreaterThan(1);
    expect(first).toBeLessThan(2);
  });
});
