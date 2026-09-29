import { clampWindow, frameTimes, GIF_FRAME_MS, GIF_MAX_LENGTH_MS } from "../videoGif";

describe("clampWindow", () => {
  it("caps the length to the limit and the clip", () => {
    expect(clampWindow(20000, 0, 9000)).toEqual({
      startMs: 0,
      lengthMs: GIF_MAX_LENGTH_MS,
    });
    expect(clampWindow(2500, 0, 5000)).toEqual({ startMs: 0, lengthMs: 2500 });
  });

  it("keeps the window inside the clip", () => {
    expect(clampWindow(10000, 8000, 5000)).toEqual({
      startMs: 5000,
      lengthMs: 5000,
    });
    expect(clampWindow(10000, -300, 2000)).toEqual({
      startMs: 0,
      lengthMs: 2000,
    });
  });
});

describe("frameTimes", () => {
  it("samples the window at the frame delay", () => {
    expect(frameTimes(1000, 500, 100)).toEqual([1000, 1100, 1200, 1300, 1400]);
  });

  it("always yields at least one frame", () => {
    expect(frameTimes(0, 30, 100)).toEqual([0]);
  });

  it("stays under the server frame limit", () => {
    expect(frameTimes(0, GIF_MAX_LENGTH_MS, GIF_FRAME_MS).length).toBeLessThanOrEqual(240);
  });
});
