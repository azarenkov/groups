import { parseScreenContent, resolveScreenContent } from "../voiceSettingsStore";

describe("resolveScreenContent", () => {
  it("takes an explicit server choice, including auto", () => {
    expect(resolveScreenContent("text", "motion")).toEqual({ value: "text", push: false });
    expect(resolveScreenContent("auto", "motion")).toEqual({ value: "auto", push: false });
  });

  it("sends a local choice when the server has never stored one", () => {
    expect(resolveScreenContent(null, "motion")).toEqual({ value: "motion", push: true });
    expect(resolveScreenContent(null, "auto")).toEqual({ value: "auto", push: false });
  });

  it("leaves the local value alone for a server that does not know this setting", () => {
    expect(resolveScreenContent(undefined, "text")).toEqual({ value: "text", push: false });
    expect(parseScreenContent("cinema")).toBeNull();
  });
});
