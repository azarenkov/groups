import { parseScreenContent, resolveScreenContent } from "../voiceSettingsStore";

describe("resolveScreenContent", () => {
  it("takes the server value once this device has synced it", () => {
    expect(resolveScreenContent("text", "motion", true)).toEqual({ value: "text", push: false });
    expect(resolveScreenContent("auto", "motion", true)).toEqual({ value: "auto", push: false });
  });

  it("keeps a local choice made before syncing and sends it to the server", () => {
    expect(resolveScreenContent("auto", "motion", false)).toEqual({ value: "motion", push: true });
  });

  it("prefers an explicit server choice over the local one", () => {
    expect(resolveScreenContent("text", "motion", false)).toEqual({ value: "text", push: false });
  });

  it("keeps the local value when the server does not know this setting yet", () => {
    expect(resolveScreenContent(undefined, "text", false)).toEqual({ value: "text", push: false });
    expect(parseScreenContent("cinema")).toBeNull();
  });
});
