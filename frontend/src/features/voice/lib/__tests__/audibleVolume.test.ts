import { audibleVolume, rememberAudibleVolume } from "../audibleVolume";

describe("audibleVolume", () => {
  it("keeps the latest volume above zero", () => {
    rememberAudibleVolume("a", 0.4);
    rememberAudibleVolume("a", 0.8);
    rememberAudibleVolume("a", 0);
    expect(audibleVolume("a")).toBe(0.8);
  });

  it("falls back to full volume for someone never heard", () => {
    expect(audibleVolume("never")).toBe(1);
  });
});
