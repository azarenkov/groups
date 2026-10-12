import { admitDmEvent, clearedAway, clearedBefore, dmClearedFor, forgetDmClears, noteDmCleared } from "../dmCleared";

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

describe("applied clears", () => {
  afterEach(() => forgetDmClears());

  it("marks messages up to the latest applied clear of a chat as cleared away", () => {
    noteDmCleared("peer", "2026-10-11T10:00:00Z");
    noteDmCleared("peer", "2026-10-11T09:00:00Z");
    expect(clearedAway("peer", "2026-10-11T09:30:00Z")).toBe(true);
    expect(clearedAway("peer", "2026-10-11T10:00:00Z")).toBe(true);
    expect(clearedAway("peer", "2026-10-11T10:00:01Z")).toBe(false);
    expect(clearedAway("other", "2026-10-11T09:30:00Z")).toBe(false);
  });

  it("forgets applied clears", () => {
    noteDmCleared("peer", "2026-10-11T10:00:00Z");
    forgetDmClears();
    expect(clearedAway("peer", "2026-10-11T09:30:00Z")).toBe(false);
  });
});

describe("admitDmEvent", () => {
  afterEach(() => forgetDmClears());

  const sent = (senderId: string, recipientId: string, occurredAt: string) => ({
    type: "dm_message.sent",
    data: { id: "m", senderId, recipientId, occurredAt },
  });

  it("drops a send from before a clear the client already applied", () => {
    const clear = {
      type: "dm_conversation.cleared",
      data: { participants: ["me", "peer"], clearedAt: "2026-10-11T10:00:00Z", scope: "everyone" },
    };
    expect(admitDmEvent(sent("peer", "me", "2026-10-11T09:59:00Z"), "me")).toBe(true);
    expect(admitDmEvent(clear, "me")).toBe(true);
    expect(admitDmEvent({ ...clear, data: { ...clear.data, participants: ["peer", "me"] } }, "me")).toBe(false);
    expect(admitDmEvent(sent("peer", "me", "2026-10-11T09:59:00Z"), "me")).toBe(false);
    expect(admitDmEvent(sent("me", "peer", "2026-10-11T09:59:30Z"), "me")).toBe(false);
    expect(admitDmEvent(sent("peer", "me", "2026-10-11T10:00:01Z"), "me")).toBe(true);
    expect(admitDmEvent(sent("other", "me", "2026-10-11T09:59:00Z"), "me")).toBe(true);
  });
});
