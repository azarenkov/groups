import { formatListTime, sortDirects } from "../directOrder";

describe("sortDirects", () => {
  it("puts the most recent conversation first, sent or received", () => {
    const items = [
      { name: "old", lastAt: "2026-09-20T10:00:00Z" },
      { name: "new", lastAt: "2026-09-27T09:00:00Z" },
      { name: "mid", lastAt: "2026-09-25T10:00:00Z" },
    ];
    expect(sortDirects(items).map((d) => d.name)).toEqual(["new", "mid", "old"]);
  });

  it("lists friends without messages after the rest, by name", () => {
    const items = [{ name: "zed" }, { name: "amy" }, { name: "chat", lastAt: "2026-09-01T00:00:00Z" }];
    expect(sortDirects(items).map((d) => d.name)).toEqual(["chat", "amy", "zed"]);
  });
});

describe("formatListTime", () => {
  const now = new Date(2026, 8, 27, 19, 30);

  it("shows the time for today", () => {
    expect(formatListTime(new Date(2026, 8, 27, 17, 41).toISOString(), now, "ru-RU")).toBe("17:41");
  });

  it("shows the weekday within a week", () => {
    const text = formatListTime(new Date(2026, 8, 24, 12, 0).toISOString(), now, "en-US");
    expect(text).toBe("Thu");
  });

  it("shows the date for older messages", () => {
    expect(formatListTime(new Date(2026, 5, 2).toISOString(), now, "en-US")).toBe("Jun 2");
    expect(formatListTime(new Date(2025, 11, 31).toISOString(), now, "en-US")).toBe("12/31/25");
  });

  it("stays empty without a message", () => {
    expect(formatListTime(undefined, now)).toBe("");
  });
});
