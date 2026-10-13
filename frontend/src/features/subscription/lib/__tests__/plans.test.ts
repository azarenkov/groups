import {
  STREAM_QUALITIES,
  bestAllowedQuality,
  hasTier,
  parseBadgeStyle,
  parseTier,
} from "../plans";

describe("plans", () => {
  it("ranks the plans", () => {
    expect(hasTier("free", "free")).toBe(true);
    expect(hasTier("free", "pro")).toBe(false);
    expect(hasTier("pro", "pro")).toBe(true);
    expect(hasTier("pro", "premium")).toBe(false);
    expect(hasTier("premium", "pro")).toBe(true);
  });

  it("falls back to free and the default style on unknown values", () => {
    expect(parseTier("gold")).toBe("free");
    expect(parseTier(undefined)).toBe("free");
    expect(parseBadgeStyle("sparkly")).toBe("tint");
    expect(parseBadgeStyle("ink")).toBe("ink");
  });

  it("keeps free on 720p and opens the rest by plan", () => {
    const open = (tier: "free" | "pro" | "premium") =>
      STREAM_QUALITIES.filter((q) => hasTier(tier, q.requires)).map((q) => q.id);
    expect(open("free")).toEqual(["720p60"]);
    expect(open("pro")).toEqual(["720p60", "1080p60", "1440p60"]);
    expect(open("premium")).toEqual(["720p60", "1080p60", "1440p60", "2160p60"]);
  });

  it("caps a saved quality that the plan no longer covers", () => {
    expect(bestAllowedQuality("2160p60", "free").id).toBe("720p60");
    expect(bestAllowedQuality("2160p60", "pro").id).toBe("1440p60");
    expect(bestAllowedQuality("2160p60", "premium").id).toBe("2160p60");
    expect(bestAllowedQuality("1440p60", "premium").id).toBe("1440p60");
  });

  it("lets auto climb only as high as the plan allows", () => {
    expect(bestAllowedQuality("auto", "free").id).toBe("720p60");
    expect(bestAllowedQuality("auto", "pro").id).toBe("1440p60");
    expect(bestAllowedQuality("auto", "premium").id).toBe("2160p60");
  });

  it("streams every quality at sixty frames", () => {
    expect(STREAM_QUALITIES.every((q) => q.frameRate === 60)).toBe(true);
  });
});

describe("bestAllowedQuality with a server ceiling", () => {
  const free = { width: 1280, height: 720, frameRate: 60, maxBitrate: 4_000_000 };
  const pro = { width: 2560, height: 1440, frameRate: 60, maxBitrate: 12_000_000 };

  it("follows the server ceiling over the local tier", () => {
    expect(bestAllowedQuality("auto", "premium", free).id).toBe("720p60");
    expect(bestAllowedQuality("2160p60", "premium", pro).id).toBe("1440p60");
    expect(bestAllowedQuality("1080p60", "free", pro).id).toBe("1080p60");
  });

  it("never asks for more than the ceiling's frame rate and bitrate", () => {
    const quality = bestAllowedQuality("auto", "premium", { ...pro, frameRate: 30, maxBitrate: 5_000_000 });
    expect(quality.frameRate).toBe(30);
    expect(quality.maxBitrate).toBe(5_000_000);
  });
});
