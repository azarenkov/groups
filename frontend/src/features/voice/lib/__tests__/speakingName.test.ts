import { speakingName } from "../speakingName";

describe("speakingName", () => {
  const participants = { me: "Alex", ann: "Ann", bob: "" };

  it("names the other person who is talking", () => {
    expect(speakingName(participants, new Set(["ann"]), "me")).toBe("Ann");
  });

  it("never names yourself", () => {
    expect(speakingName(participants, new Set(["me"]), "me")).toBeNull();
  });

  it("falls back to the id when the name is unknown", () => {
    expect(speakingName(participants, new Set(["bob"]), "me")).toBe("bob");
  });

  it("stays quiet when nobody talks", () => {
    expect(speakingName(participants, new Set(), "me")).toBeNull();
  });
});
