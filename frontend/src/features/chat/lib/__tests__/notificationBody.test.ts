import { notificationBody } from "../notificationBody";

describe("notificationBody", () => {
  it("keeps text as it is", () => {
    expect(notificationBody("hi", [])).toBe("hi");
  });

  it("names a voice message with its length", () => {
    const body = notificationBody("", [{ voice: { durationMs: 42_000, waveform: [] } }]);
    expect(body).toContain("0:42");
  });

  it("falls back to an attachment label and then to nothing", () => {
    expect(notificationBody("", [{ id: "a" }])).not.toBe("");
    expect(notificationBody(undefined, undefined)).toBe("");
  });
});
