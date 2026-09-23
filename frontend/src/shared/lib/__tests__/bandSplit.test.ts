import {
  DECIMATION,
  LOWPASS_TAPS,
  bandReconstruction,
  designLowpass,
  firFilter,
  responseAt,
} from "../bandSplit";

const RATE = 48000;
const filter = designLowpass(LOWPASS_TAPS, 7200, RATE);

describe("the band splitting filter", () => {
  it("passes the voice band untouched", () => {
    expect(responseAt(filter, 100, RATE)).toBeCloseTo(1, 2);
    expect(responseAt(filter, 3000, RATE)).toBeCloseTo(1, 1);
  });

  it("stops what the model cannot hear", () => {
    expect(responseAt(filter, 12000, RATE)).toBeLessThan(0.02);
    expect(responseAt(filter, 16000, RATE)).toBeLessThan(0.02);
  });

  it("is symmetric, so it delays every frequency the same", () => {
    for (let i = 0; i < LOWPASS_TAPS; i++) {
      expect(filter[i]).toBeCloseTo(filter[LOWPASS_TAPS - 1 - i], 12);
    }
  });

  it("keeps unity gain overall", () => {
    let sum = 0;
    for (const value of filter) sum += value;
    expect(sum).toBeCloseTo(1, 6);
  });

  it("decimates by three, which is 48 kHz down to the model rate", () => {
    expect(RATE / DECIMATION).toBe(16000);
  });
});

const tone = (hz: number, length: number): Float32Array => {
  const out = new Float32Array(length);
  for (let i = 0; i < length; i++) out[i] = Math.sin((2 * Math.PI * hz * i) / RATE);
  return out;
};

const peak = (samples: Float32Array, from: number): number => {
  let max = 0;
  for (let i = from; i < samples.length; i++) max = Math.max(max, Math.abs(samples[i]));
  return max;
};

describe("splitting and putting the signal back together", () => {
  const settle = LOWPASS_TAPS * 4;

  it("returns the voice band unchanged", () => {
    const input = tone(400, 2048);
    const out = bandReconstruction(input, filter, DECIMATION);
    expect(peak(out, settle)).toBeCloseTo(1, 2);
  });

  it("leaves no notch at the crossover", () => {
    const input = tone(7200, 2048);
    const out = bandReconstruction(input, filter, DECIMATION);
    expect(peak(out, settle)).toBeGreaterThan(0.97);
  });

  it("carries the band the model never sees", () => {
    const input = tone(12000, 2048);
    const out = bandReconstruction(input, filter, DECIMATION);
    expect(peak(out, settle)).toBeGreaterThan(0.97);
  });

  it("delays every band by the same amount", () => {
    const input = tone(1000, 2048);
    const out = bandReconstruction(input, filter, DECIMATION);
    const delay = LOWPASS_TAPS - 1;
    for (let i = settle; i < 2000; i++) {
      expect(out[i]).toBeCloseTo(input[i - delay], 5);
    }
  });

  it("filters without inventing energy", () => {
    const filtered = firFilter(tone(12000, 1024), filter);
    expect(peak(filtered, settle)).toBeLessThan(0.02);
  });
});
