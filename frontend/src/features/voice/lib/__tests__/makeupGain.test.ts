import {
  MAKEUP_BASE,
  MAKEUP_MAX,
  isSpeech,
  makeupGainFor,
  speechBandRatio,
} from "../makeupGain";

const SAMPLE_RATE = 48000;
const BINS = 256;

const spectrum = (fill: (hz: number) => number): Uint8Array => {
  const data = new Uint8Array(BINS);
  const binHz = SAMPLE_RATE / 2 / BINS;
  for (let i = 0; i < BINS; i++) data[i] = fill((i + 0.5) * binHz);
  return data;
};

const voice = spectrum((hz) => (hz >= 300 && hz <= 3400 ? 200 : 20));
const rumble = spectrum((hz) => (hz < 300 ? 200 : 10));
const hiss = spectrum(() => 60);

describe("speechBandRatio", () => {
  it("is high for energy inside the speech band", () => {
    expect(speechBandRatio(voice, SAMPLE_RATE)).toBeGreaterThan(0.9);
  });

  it("is low for low-frequency rumble", () => {
    expect(speechBandRatio(rumble, SAMPLE_RATE)).toBeLessThan(0.2);
  });

  it("is low for broadband hiss", () => {
    expect(speechBandRatio(hiss, SAMPLE_RATE)).toBeLessThan(0.45);
  });

  it("is zero for silence", () => {
    expect(speechBandRatio(new Uint8Array(BINS), SAMPLE_RATE)).toBe(0);
  });
});

describe("isSpeech", () => {
  it("accepts speech-shaped sound while the gate is open", () => {
    expect(isSpeech(0.2, speechBandRatio(voice, SAMPLE_RATE), true)).toBe(true);
  });

  it("rejects rumble and hiss", () => {
    expect(isSpeech(0.2, speechBandRatio(rumble, SAMPLE_RATE), true)).toBe(false);
    expect(isSpeech(0.2, speechBandRatio(hiss, SAMPLE_RATE), true)).toBe(false);
  });

  it("rejects everything while the gate is closed", () => {
    expect(isSpeech(0.2, 0.9, false)).toBe(false);
  });

  it("rejects input below the absolute floor", () => {
    expect(isSpeech(0.005, 0.9, true)).toBe(false);
  });
});

describe("makeupGainFor", () => {
  it("never drops below the constant restoration", () => {
    let gain = MAKEUP_BASE;
    for (let i = 0; i < 400; i++) gain = makeupGainFor(0.001, 0.0002, gain, false);
    expect(gain).toBeCloseTo(MAKEUP_BASE, 2);
  });

  it("returns to the constant restoration after speech stops", () => {
    let gain = MAKEUP_MAX;
    for (let i = 0; i < 400; i++) gain = makeupGainFor(0.001, 0.0002, gain, false);
    expect(gain).toBeCloseTo(MAKEUP_BASE, 2);
  });

  it("leaves steady noise at the constant restoration", () => {
    const ratio = speechBandRatio(hiss, SAMPLE_RATE);
    let gain = MAKEUP_BASE;
    for (let i = 0; i < 200; i++) {
      gain = makeupGainFor(0.02, 0.004, gain, isSpeech(0.02, ratio, true));
    }
    expect(gain).toBeCloseTo(MAKEUP_BASE, 2);
  });

  it("follows the attenuation the denoiser introduced during speech", () => {
    let gain = MAKEUP_BASE;
    for (let i = 0; i < 60; i++) gain = makeupGainFor(0.2, 0.067, gain, true);
    expect(gain).toBeCloseTo(3, 1);
  });

  it("never exceeds the ceiling", () => {
    let gain = MAKEUP_BASE;
    for (let i = 0; i < 200; i++) gain = makeupGainFor(0.5, 0.001, gain, true);
    expect(gain).toBeLessThanOrEqual(MAKEUP_MAX);
  });

  it("releases rather than freezing when the denoiser goes silent", () => {
    expect(makeupGainFor(0.2, 0, MAKEUP_MAX, true)).toBeLessThan(MAKEUP_MAX);
  });
});
