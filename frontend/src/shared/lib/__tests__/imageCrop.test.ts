import { appendCrop, clampOffset, coverScale, cropFromView, cropOf, setCrop } from "../imageCrop";

const view = (overrides: Partial<Parameters<typeof cropFromView>[0]> = {}) => ({
  frameWidth: 300,
  frameHeight: 300,
  naturalWidth: 1200,
  naturalHeight: 600,
  zoom: 1,
  offsetX: 0,
  offsetY: 0,
  ...overrides,
});

describe("image crop", () => {
  it("scales the picture until it covers the frame", () => {
    expect(coverScale(view())).toBe(0.5);
    expect(coverScale(view({ naturalWidth: 600, naturalHeight: 1200 }))).toBe(0.5);
  });

  it("never lets the frame leave the picture", () => {
    expect(clampOffset(view({ offsetX: 50, offsetY: 50 }))).toEqual({ offsetX: 0, offsetY: 0 });
    expect(clampOffset(view({ offsetX: -9999, offsetY: -9999 }))).toEqual({ offsetX: -300, offsetY: 0 });
  });

  it("turns the view into an area in picture pixels", () => {
    expect(cropFromView(view())).toEqual({ x: 0, y: 0, width: 600, height: 600 });
    expect(cropFromView(view({ offsetX: -300 }))).toEqual({ x: 600, y: 0, width: 600, height: 600 });
    expect(cropFromView(view({ zoom: 2, offsetX: -300, offsetY: -150 }))).toEqual({
      x: 300,
      y: 150,
      width: 300,
      height: 300,
    });
  });

  it("keeps a banner area at three to one", () => {
    const rect = cropFromView(
      view({ frameWidth: 480, frameHeight: 160, naturalWidth: 3000, naturalHeight: 2000 }),
    );
    expect(rect.width / rect.height).toBeCloseTo(3, 1);
    expect(rect.x + rect.width).toBeLessThanOrEqual(3000);
    expect(rect.y + rect.height).toBeLessThanOrEqual(2000);
  });

  it("travels with the file and lands in the form", () => {
    const file = new File(["x"], "a.png", { type: "image/png" });
    const form = new FormData();
    appendCrop(form, file);
    expect(form.get("cropX")).toBeNull();

    setCrop(file, { x: 10, y: 20, width: 100, height: 100 });
    expect(cropOf(file)).toEqual({ x: 10, y: 20, width: 100, height: 100 });
    appendCrop(form, file);
    expect(form.get("cropX")).toBe("10");
    expect(form.get("cropY")).toBe("20");
    expect(form.get("cropWidth")).toBe("100");
    expect(form.get("cropHeight")).toBe("100");
  });
});
