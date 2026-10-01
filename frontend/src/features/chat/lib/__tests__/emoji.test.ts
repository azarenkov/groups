import { emojiOnlyCount, isJumboEmoji } from "../emoji";

describe("isJumboEmoji", () => {
  it("enlarges a single emoji, including composed ones", () => {
    for (const text of ["👍", " 👍🏽 ", "👨‍👩‍👧", "🇷🇺", "❤️", "1️⃣", "🏳️‍🌈"]) {
      expect(isJumboEmoji(text)).toBe(true);
    }
  });

  it("keeps text, mixed messages and several emoji at normal size", () => {
    for (const text of ["", "ок", "👍 ок", "👍👍", "©", "123", "❤"]) {
      expect(isJumboEmoji(text)).toBe(false);
    }
  });

  it("counts emoji-only messages", () => {
    expect(emojiOnlyCount("😀 😃 😄")).toBe(3);
    expect(emojiOnlyCount("😀 a")).toBe(0);
  });
});
