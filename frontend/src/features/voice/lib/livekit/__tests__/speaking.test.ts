import {
  SPEAKING_HOLD_MS,
  SPEAKING_RMS,
  speakingFor,
} from "../remoteAudio";

describe("speakingFor", () => {
  it("lights up as soon as the level crosses the threshold", () => {
    const state = speakingFor(SPEAKING_RMS, 1000, 0);
    expect(state.speaking).toBe(true);
    expect(state.holdUntil).toBe(1000 + SPEAKING_HOLD_MS);
  });

  it("stays dark on silence", () => {
    expect(speakingFor(0, 1000, 0).speaking).toBe(false);
  });

  it("stays dark on a level below the threshold", () => {
    expect(speakingFor(SPEAKING_RMS / 2, 1000, 0).speaking).toBe(false);
  });

  it("holds through a short gap between words", () => {
    const loud = speakingFor(0.2, 1000, 0);
    const gap = speakingFor(0.001, 1000 + SPEAKING_HOLD_MS - 50, loud.holdUntil);
    expect(gap.speaking).toBe(true);
    expect(gap.holdUntil).toBe(loud.holdUntil);
  });

  it("goes dark once the hold expires", () => {
    const loud = speakingFor(0.2, 1000, 0);
    expect(speakingFor(0.001, loud.holdUntil, loud.holdUntil).speaking).toBe(false);
  });

  it("extends the hold while the talking continues", () => {
    const first = speakingFor(0.2, 1000, 0);
    const second = speakingFor(0.2, 1200, first.holdUntil);
    expect(second.holdUntil).toBeGreaterThan(first.holdUntil);
  });
});
