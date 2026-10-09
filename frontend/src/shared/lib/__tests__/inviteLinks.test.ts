import { authRedirectTarget, extractPackSlug, extractResetToken, resetPasswordDeepLink } from "../inviteLinks";

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

describe("password reset links", () => {
  const token = `${"a".repeat(32)}.${"0123456789abcdef".repeat(4)}`;

  it("round-trips a reset token through the desktop link", () => {
    expect(extractResetToken(resetPasswordDeepLink(token))).toBe(token);
    expect(extractResetToken(`rift://reset-password/?token=${token}`)).toBe(token);
  });

  it("ignores anything that is not a reset token", () => {
    expect(extractResetToken("rift://reset-password?token=abc")).toBeNull();
    expect(extractResetToken(`rift://join/${"a".repeat(22)}`)).toBeNull();
    expect(extractResetToken(`https://rift.example/reset-password?token=${token}`)).toBeNull();
  });
});
