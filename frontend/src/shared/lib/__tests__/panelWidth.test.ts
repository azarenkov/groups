import { MIN_CENTER_WIDTH, clampPanelWidth } from "../panelWidth";

const bounds = { min: 240, max: 420 };

describe("clampPanelWidth", () => {
  it("keeps a width inside the panel's limits", () => {
    expect(clampPanelWidth(300, bounds, 1600, 240)).toBe(300);
    expect(clampPanelWidth(100, bounds, 1600, 240)).toBe(240);
    expect(clampPanelWidth(900, bounds, 1600, 240)).toBe(420);
  });

  it("leaves the conversation at least its minimum width", () => {
    const viewport = 240 + MIN_CENTER_WIDTH + 300;
    expect(clampPanelWidth(420, bounds, viewport, 240)).toBe(300);
  });

  it("never goes below the minimum even in a tiny window", () => {
    expect(clampPanelWidth(400, bounds, 700, 240)).toBe(240);
  });

  it("rounds to whole pixels", () => {
    expect(clampPanelWidth(301.6, bounds, 1600, 240)).toBe(302);
  });
});
