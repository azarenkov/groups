import {
  CPU_LIMIT_HOLD_MS,
  chooseCodec,
  contentHintFor,
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

describe("the content hint", () => {
  it("keeps text sharp even at sixty frames", () => {
    expect(contentHintFor("text", 60)).toBe("detail");
  });

  it("keeps motion smooth when asked", () => {
    expect(contentHintFor("motion", 30)).toBe("motion");
  });

  it("guesses from the frame rate when left alone", () => {
    expect(contentHintFor("auto", 60)).toBe("motion");
    expect(contentHintFor("auto", 30)).toBe("detail");
  });
});

describe("falling back to vp9", () => {
  const sample = (at: number, cpuLimited: boolean, fps: number) => ({
    at,
    cpuLimited,
    framesPerSecond: fps,
  });

  it("leaves a healthy encoder alone", () => {
    const samples = [sample(0, false, 60), sample(5000, false, 58)];
    expect(shouldFallBack(samples, 60, 11000)).toBe(false);
  });

  it("waits out a short hiccup", () => {
    const samples = [sample(0, false, 60), sample(1000, true, 20)];
    expect(shouldFallBack(samples, 60, 5000)).toBe(false);
  });

  it("gives up after the encoder struggles long enough", () => {
    const samples = [sample(0, true, 20), sample(2000, true, 18)];
    expect(shouldFallBack(samples, 60, CPU_LIMIT_HOLD_MS)).toBe(true);
  });

  it("counts a frame rate collapse as struggling", () => {
    const samples = [sample(0, false, 20), sample(3000, false, 22)];
    expect(shouldFallBack(samples, 60, CPU_LIMIT_HOLD_MS + 1)).toBe(true);
  });

  it("restarts the clock once the encoder recovers", () => {
    const samples = [
      sample(0, true, 20),
      sample(4000, false, 60),
      sample(6000, true, 20),
    ];
    expect(shouldFallBack(samples, 60, 12000)).toBe(false);
  });
});
