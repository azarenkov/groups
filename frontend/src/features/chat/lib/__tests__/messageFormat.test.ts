import type { ChannelMessage } from "../../../../shared/types/channel";
import {
  formatCallDuration,
  formatFileSize,
  mergeWithLive,
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
    expect(formatCallDuration(45)).toContain("45");
    expect(formatCallDuration(3600)).toBeTruthy();
  });

  it("compares calendar days", () => {
    expect(sameDay(new Date("2026-01-01T23:00:00"), new Date("2026-01-01T01:00:00"))).toBe(true);
    expect(sameDay(new Date("2026-01-01T23:00:00"), new Date("2026-01-02T01:00:00"))).toBe(false);
  });
});
