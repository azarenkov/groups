import type { ChannelMessage } from "../../../../shared/types/channel";
import {
  formatCallDuration,
  formatFileSize,
  mergeWithLive,
  reconcileSyncedWindow,
  sameDay,
  splitForSend,
} from "../messageFormat";

const message = (id: string, createdAt: string): ChannelMessage =>
  ({ id, createdAt }) as ChannelMessage;

describe("mergeWithLive", () => {
  it("returns history untouched when it already covers the live messages", () => {
    const history = [message("a", "2026-01-01T10:00:00Z")];
    expect(mergeWithLive(history, [message("a", "2026-01-01T10:00:00Z")])).toBe(history);
  });

  it("keeps live messages that history does not contain, ordered by time", () => {
    const history = [message("a", "2026-01-01T10:00:00Z")];
    const live = [message("b", "2026-01-01T09:00:00Z")];
    expect(mergeWithLive(history, live).map((m) => m.id)).toEqual(["b", "a"]);
  });
});

describe("splitForSend", () => {
  it("leaves short text as one chunk", () => {
    expect(splitForSend("hello", 10)).toEqual(["hello"]);
  });

  it("splits on a line break close to the limit", () => {
    const text = "a".repeat(180) + "\n" + "b".repeat(60);
    const chunks = splitForSend(text, 200);
    expect(chunks).toEqual(["a".repeat(180), "b".repeat(60)]);
  });

  it("never exceeds the limit", () => {
    const chunks = splitForSend("word ".repeat(400), 200);
    expect(chunks.every((c) => c.length <= 200)).toBe(true);
    expect(chunks.join(" ").replace(/\s+/g, " ").trim()).toBe(
      "word ".repeat(400).trim(),
    );
  });
});

describe("formatting helpers", () => {
  it("formats file sizes by unit", () => {
    expect(formatFileSize(512)).toBe("512 B");
    expect(formatFileSize(2048)).toBe("2.0 KB");
    expect(formatFileSize(5 * 1024 * 1024)).toBe("5.0 MB");
  });

  it("formats call durations", () => {
    expect(formatCallDuration(45)).toBe("0:45");
    expect(formatCallDuration(12 * 60 + 4)).toBe("12:04");
    expect(formatCallDuration(3600 + 2 * 60 + 15)).toBe("1:02:15");
    expect(formatCallDuration(-5)).toBe("0:00");
  });

  it("compares calendar days", () => {
    expect(sameDay(new Date("2026-01-01T23:00:00"), new Date("2026-01-01T01:00:00"))).toBe(true);
    expect(sameDay(new Date("2026-01-01T23:00:00"), new Date("2026-01-02T01:00:00"))).toBe(false);
  });
});

describe("reconcileSyncedWindow", () => {
  const a = message("a", "2026-01-01T10:00:00Z");
  const b = message("b", "2026-01-01T10:01:00Z");
  const c = message("c", "2026-01-01T10:02:00Z");
  const live = message("live", "2026-01-01T10:03:00Z");

  it("drops everything the server no longer has once the whole history was read", () => {
    const known = new Set(["a", "b", "c"]);
    expect(reconcileSyncedWindow([], [a, b, c, live], true, known)).toEqual([live]);
  });

  it("drops missing messages only inside the synced window", () => {
    const known = new Set(["a", "b", "c"]);
    expect(reconcileSyncedWindow([a, c], [a, b, c], false, known)).toEqual([a, c]);
    expect(reconcileSyncedWindow([b], [a, b, c], false, known)).toEqual([a, b]);
  });
});
