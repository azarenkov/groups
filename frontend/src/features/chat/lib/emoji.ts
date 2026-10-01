const MODIFIED = "\\p{Emoji_Modifier_Base}\\uFE0F?\\p{Emoji_Modifier}";
const PICTOGRAPH = "(?:\\p{Emoji_Presentation}(?!\\uFE0E)|\\p{Extended_Pictographic}\\uFE0F)";
const ELEMENT = `(?:${MODIFIED}|${PICTOGRAPH})`;
const JOINED = `(?:${MODIFIED}|(?:\\p{Emoji_Presentation}|\\p{Extended_Pictographic})\\uFE0F?)`;
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
