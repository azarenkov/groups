const MODIFIED = "\\p{Emoji_Modifier_Base}\\uFE0F?\\p{Emoji_Modifier}";
const PICTOGRAPH = "(?:\\p{Emoji_Presentation}(?!\\uFE0E)|\\p{Extended_Pictographic}\\uFE0F)";
const ELEMENT = `(?:${MODIFIED}|${PICTOGRAPH})`;
const PEOPLE = "\\u{1F466}-\\u{1F469}\\u{1F9D1}\\u{1F9D2}\\u{1FAF2}";
const COMPONENTS = [
  PEOPLE,
  "\\u{1F48B}\\u2764\\u{1F91D}\\u2640\\u2642\\u26A7\\u{1F9B0}-\\u{1F9B3}",
  "\\u2695\\u2696\\u2708\\u{1F33E}\\u{1F373}\\u{1F37C}\\u{1F384}\\u{1F393}\\u{1F3A4}\\u{1F3A8}\\u{1F3EB}\\u{1F3ED}",
  "\\u{1F4BB}\\u{1F4BC}\\u{1F527}\\u{1F52C}\\u{1F680}\\u{1F692}\\u{1F9AF}\\u{1F9BC}\\u{1F9BD}\\u{1FA70}",
  "\\u27A1\\u2194\\u2195\\u{1F308}\\u2620\\u{1F5E8}\\u{1F32B}\\u{1F4A8}\\u{1F4AB}\\u{1F525}\\u{1FA79}\\u2744\\u2B1B\\u{1F7E9}\\u{1F7EB}\\u{1F4A5}",
].join("");
const JOINED = `(?:[${PEOPLE}]\\p{Emoji_Modifier}|[${COMPONENTS}]\\uFE0F?)`;
const SEQUENCE = `${ELEMENT}(?:\\u200D${JOINED})*`;
const FLAG = "\\p{Regional_Indicator}{2}";
const KEYCAP = "[#*0-9]\\uFE0F?\\u20E3";
const TAG_FLAG = "\\u{1F3F4}[\\u{E0020}-\\u{E007E}]+\\u{E007F}";

const EMOJI_GRAPHEME = new RegExp(`^(?:${FLAG}|${KEYCAP}|${TAG_FLAG}|${SEQUENCE})$`, "u");

const segmenter =
  typeof Intl !== "undefined" && "Segmenter" in Intl
    ? new Intl.Segmenter(undefined, { granularity: "grapheme" })
    : null;

export const isEmojiGrapheme = (grapheme: string): boolean => EMOJI_GRAPHEME.test(grapheme);

export const emojiOnlyCount = (text: string): number => {
  const trimmed = text.trim();
  if (!trimmed || !segmenter) return 0;
  let count = 0;
  for (const { segment } of segmenter.segment(trimmed)) {
    if (/^\s+$/u.test(segment)) continue;
    if (!isEmojiGrapheme(segment)) return 0;
    count += 1;
  }
  return count;
};

export const isJumboEmoji = (text: string): boolean => emojiOnlyCount(text) === 1;
