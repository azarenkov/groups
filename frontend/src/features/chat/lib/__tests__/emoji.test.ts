import { emojiOnlyCount, isJumboEmoji } from "../emoji";

describe("isJumboEmoji", () => {
  it("enlarges a single emoji, including composed ones", () => {
    for (const text of ["👍", " 👍🏽 ", "👨‍👩‍👧", "🇷🇺", "❤️", "1️⃣", "🏳️‍🌈", "☝🏽", "🏴\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}", "🧑🏽‍💻"]) {
      expect(isJumboEmoji(text)).toBe(true);
    }
  });

  it("keeps text, mixed messages and several emoji at normal size", () => {
    for (const text of ["", "ок", "👍 ок", "👍👍", "©", "123", "❤", "☕\uFE0E", "A\u20E3", "7"]) {
      expect(isJumboEmoji(text)).toBe(false);
    }
  });

  it("counts emoji-only messages", () => {
    expect(emojiOnlyCount("😀 😃 😄")).toBe(3);
    expect(emojiOnlyCount("😀 a")).toBe(0);
  });
});
