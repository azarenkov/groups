import { parseNoiseMode } from "../voiceSettingsStore";

describe("parseNoiseMode", () => {
  it("keeps high suppression from the server whatever the plan", () => {
    expect(parseNoiseMode("high")).toBe("high");
  });

  it("turns the retired off setting into standard", () => {
    expect(parseNoiseMode("off")).toBe("standard");
  });

  it("falls back to standard for anything else", () => {
    expect(parseNoiseMode("standard")).toBe("standard");
    expect(parseNoiseMode("ultra")).toBe("standard");
    expect(parseNoiseMode(undefined)).toBe("standard");
  });
});
