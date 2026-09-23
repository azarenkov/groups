import {
  DECIMATION,
  LOWPASS_TAPS,
  designLowpass,
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
