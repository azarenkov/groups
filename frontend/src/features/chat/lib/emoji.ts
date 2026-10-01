const EMOJI_GRAPHEME = /\p{Emoji_Presentation}|\p{Extended_Pictographic}️|\p{Regional_Indicator}{2}|⃣/u;

const segmenter =
  typeof Intl !== "undefined" && "Segmenter" in Intl
    ? new Intl.Segmenter(undefined, { granularity: "grapheme" })
    : null;

export const emojiOnlyCount = (text: string): number => {
  const trimmed = text.trim();
  if (!trimmed || !segmenter) return 0;
  let count = 0;
  for (const { segment } of segmenter.segment(trimmed)) {
    if (/^\s+$/u.test(segment)) continue;
    if (!EMOJI_GRAPHEME.test(segment)) return 0;
    count += 1;
  }
  return count;
};

export const isJumboEmoji = (text: string): boolean => emojiOnlyCount(text) === 1;
