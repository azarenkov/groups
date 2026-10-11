import { clearedBefore, dmClearedFor } from "../dmCleared";

it("resolves the peer of a cleared conversation", () => {
  const data = { participants: ["me", "peer"], clearedAt: "2026-10-11T10:00:00Z", scope: "everyone" };
  expect(dmClearedFor(data, "me")).toEqual({ peerId: "peer", clearedAt: "2026-10-11T10:00:00Z", scope: "everyone" });
  expect(dmClearedFor({ ...data, participants: ["peer", "me"] }, "me")?.peerId).toBe("peer");
});

it("ignores events for other conversations or without a timestamp", () => {
  expect(dmClearedFor({ participants: ["a", "b"], clearedAt: "x" }, "me")).toBeNull();
  expect(dmClearedFor({ participants: ["me", "peer"] }, "me")).toBeNull();
  expect(dmClearedFor({ participants: ["me", "peer"], clearedAt: "x" }, undefined)).toBeNull();
  expect(dmClearedFor(null, "me")).toBeNull();
});

it("defaults to clearing only for me", () => {
  expect(dmClearedFor({ participants: ["me", "peer"], clearedAt: "t" }, "me")?.scope).toBe("me");
});

it("compares message time with the clear time", () => {
  expect(clearedBefore("2026-10-11T09:59:59Z", "2026-10-11T10:00:00Z")).toBe(true);
  expect(clearedBefore("2026-10-11T10:00:00Z", "2026-10-11T10:00:00Z")).toBe(true);
  expect(clearedBefore("2026-10-11T10:00:01Z", "2026-10-11T10:00:00Z")).toBe(false);
  expect(clearedBefore("bad", "2026-10-11T10:00:00Z")).toBe(false);
});

it("tells apart a message and a clear within the same millisecond", () => {
  expect(clearedBefore("2026-10-11T10:00:00.123457Z", "2026-10-11T10:00:00.123456Z")).toBe(false);
  expect(clearedBefore("2026-10-11T10:00:00.123455Z", "2026-10-11T10:00:00.123456Z")).toBe(true);
  expect(clearedBefore("2026-10-11T10:00:00.123456789Z", "2026-10-11T10:00:00.123456Z")).toBe(false);
  expect(clearedBefore("2026-10-11T10:00:00.123Z", "2026-10-11T10:00:00.123456Z")).toBe(true);
});
