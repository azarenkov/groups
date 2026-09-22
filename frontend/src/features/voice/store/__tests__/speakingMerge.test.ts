import { mergeSpeaking } from "../speakingStore";

const ids = (...values: string[]) => new Set(values);

describe("mergeSpeaking", () => {
  it("keeps the reported state for participants nobody measures", () => {
    const merged = mergeSpeaking(ids("alice"), ids("alice"), ids("bob"), null, false);
    expect(Array.from(merged).sort()).toEqual(["alice", "bob"]);
  });

  it("lets the measurement win for a participant it tracks", () => {
    const merged = mergeSpeaking(ids(), ids("alice"), ids("alice"), null, false);
    expect(merged.size).toBe(0);
  });

  it("falls back to the reported state when nothing is measured", () => {
    const merged = mergeSpeaking(ids(), ids(), ids("alice", "bob"), null, false);
    expect(Array.from(merged).sort()).toEqual(["alice", "bob"]);
  });

  it("adds yourself while your microphone is open", () => {
    expect(mergeSpeaking(ids(), ids(), ids(), "me", true).has("me")).toBe(true);
  });

  it("leaves you out while your microphone is quiet", () => {
    expect(mergeSpeaking(ids(), ids(), ids(), "me", false).has("me")).toBe(false);
  });

  it("ignores local speech without an identity", () => {
    expect(mergeSpeaking(ids(), ids(), ids(), null, true).size).toBe(0);
  });
});
