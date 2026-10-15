import { extractInviteToken, extractProfileLinkId, authRedirectTarget, extractPackSlug, extractResetToken, resetPasswordDeepLink, extractProfileId, extractProfileDeepLink, profileDeepLink } from "../inviteLinks";

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

describe("profile links", () => {
  const id = "0b6f3a1e-6a51-4c7e-9a0e-5f1b2c3d4e5f";

  it("accepts user ids only, in any case", () => {
    expect(extractProfileId(id.toUpperCase())).toBe(id);
    expect(extractProfileId("alice")).toBeNull();
    expect(extractProfileId(`${id}x`)).toBeNull();
  });

  it("returns to a profile after signing in", () => {
    expect(authRedirectTarget(`?next=${encodeURIComponent(`/u/${id}`)}`)).toBe(`/u/${id}`);
    expect(authRedirectTarget(`?next=${encodeURIComponent("/u/alice")}`)).toBe("/app");
  });

  it("round-trips a user id through the desktop link", () => {
    expect(extractProfileDeepLink(profileDeepLink(id))).toBe(id);
    expect(extractProfileDeepLink(`rift://u/${id}/`)).toBe(id);
    expect(extractProfileDeepLink(`rift://join/${id}`)).toBeNull();
    expect(extractProfileDeepLink("rift://u/alice")).toBeNull();
  });
});

describe("profile links in search", () => {
  const id = "60468fcd-88b9-4633-902c-3124191a8505";

  it("reads the profile id from web, desktop and bare links", () => {
    expect(extractProfileLinkId(`https://rift.localtest.me/u/${id}`)).toBe(id);
    expect(extractProfileLinkId(` https://rift.localtest.me/u/${id.toUpperCase()}/?ref=x `)).toBe(id);
    expect(extractProfileLinkId(`rift://u/${id}`)).toBe(id);
    expect(extractProfileLinkId(id)).toBe(id);
    expect(extractProfileLinkId("https://rift.localtest.me/u/not-a-user")).toBeNull();
    expect(extractProfileLinkId("alex")).toBeNull();
  });

  it("does not mistake a profile link for a community invite", () => {
    expect(extractInviteToken(`https://rift.localtest.me/u/${id}`)).toBeNull();
    expect(extractInviteToken(`rift://u/${id}`)).toBeNull();
    expect(extractInviteToken(`https://rift.localtest.me/join/${id}`)).toBe(id);
    expect(extractInviteToken(id)).toBe(id);
    expect(extractInviteToken("https://rift.localtest.me/join/AbCdEfGhIjKlMnOpQrStUvWxYz012345")).toBe(
      "AbCdEfGhIjKlMnOpQrStUvWxYz012345",
    );
  });
});
