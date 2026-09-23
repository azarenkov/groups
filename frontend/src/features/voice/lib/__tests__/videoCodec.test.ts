import {
  CPU_LIMIT_HOLD_MS,
  chooseCodec,
  degradationFor,
  shouldFallBack,
} from "../videoCodec";

const efficient = { supported: true, smooth: true, powerEfficient: true };
const software = { supported: true, smooth: true, powerEfficient: false };

describe("choosing the screen codec", () => {
  it("stays on vp9 in a browser", () => {
    expect(chooseCodec(false, efficient)).toBe("vp9");
  });

  it("takes av1 on a desktop that encodes it in hardware", () => {
    expect(chooseCodec(true, efficient)).toBe("av1");
  });

  it("refuses av1 when encoding would be software", () => {
    expect(chooseCodec(true, software)).toBe("vp9");
  });

  it("refuses av1 when the machine says nothing", () => {
    expect(chooseCodec(true, null)).toBe("vp9");
  });
});

describe("what the encoder gives up first", () => {
  it("keeps text sharp and drops frames instead", () => {
    expect(degradationFor("text", true)).toBe("maintain-resolution");
  });

  it("keeps motion smooth and drops resolution instead", () => {
    expect(degradationFor("motion", false)).toBe("maintain-framerate");
  });

  it("balances both when quality is automatic", () => {
    expect(degradationFor("auto", true)).toBe("balanced");
  });

  it("holds a hand-picked resolution", () => {
    expect(degradationFor("auto", false)).toBe("maintain-resolution");
  });
});

describe("falling back to vp9", () => {
  const sample = (
    at: number,
    cpuLimited: boolean,
    fps: number,
    sourceFps: number | null = 60,
  ) => ({
    at,
    cpuLimited,
    framesPerSecond: fps,
    sourceFramesPerSecond: sourceFps,
  });

  it("leaves a healthy encoder alone", () => {
    const samples = [sample(0, false, 60), sample(5000, false, 58)];
    expect(shouldFallBack(samples, 11000)).toBe(false);
  });

  it("waits out a short hiccup", () => {
    const samples = [sample(0, false, 60), sample(1000, true, 20)];
    expect(shouldFallBack(samples, 5000)).toBe(false);
  });

  it("gives up after the encoder struggles long enough", () => {
    const samples = [sample(0, true, 20), sample(2000, true, 18)];
    expect(shouldFallBack(samples, CPU_LIMIT_HOLD_MS)).toBe(true);
  });

  it("counts frames the encoder drops from the source as struggling", () => {
    const samples = [sample(0, false, 20), sample(3000, false, 22)];
    expect(shouldFallBack(samples, CPU_LIMIT_HOLD_MS + 1)).toBe(true);
  });

  it("does not blame the encoder for a still screen", () => {
    const samples = [sample(0, false, 3, 3), sample(6000, false, 1, 1)];
    expect(shouldFallBack(samples, CPU_LIMIT_HOLD_MS + 1)).toBe(false);
  });

  it("trusts only the cpu flag when the source rate is unknown", () => {
    const samples = [sample(0, false, 5, null), sample(6000, false, 5, null)];
    expect(shouldFallBack(samples, CPU_LIMIT_HOLD_MS + 1)).toBe(false);
  });

  it("restarts the clock once the encoder recovers", () => {
    const samples = [
      sample(0, true, 20),
      sample(4000, false, 60),
      sample(6000, true, 20),
    ];
    expect(shouldFallBack(samples, 12000)).toBe(false);
  });
});
