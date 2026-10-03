import { authRedirectTarget, extractPackSlug } from "../inviteLinks";

describe("sticker pack links", () => {
  it("accepts pack slugs only", () => {
    expect(extractPackSlug("Cats_2026")).toBe("cats_2026");
    expect(extractPackSlug("ca")).toBeNull();
    expect(extractPackSlug("../app")).toBeNull();
  });

  it("returns to a pack link after signing in", () => {
    expect(authRedirectTarget("?next=%2Fstickers%2Fcats")).toBe("/stickers/cats");
    expect(authRedirectTarget("?next=%2Fstickers%2F..%2Fadmin")).toBe("/app");
    expect(authRedirectTarget("?next=%2Fsettings")).toBe("/app");
  });
});
