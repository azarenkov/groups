const MODIFIED = "\\p{Emoji_Modifier_Base}\\uFE0F?\\p{Emoji_Modifier}";
const PICTOGRAPH = "(?:\\p{Emoji_Presentation}(?!\\uFE0E)|\\p{Extended_Pictographic}\\uFE0F)";
const ELEMENT = `(?:${MODIFIED}|${PICTOGRAPH})`;
const TONE = "\\p{Emoji_Modifier}";
const VS = "\\uFE0F?";
const ZWJ = "\\u200D";
const PERSON = `[\\u{1F466}-\\u{1F469}\\u{1F9D1}\\u{1F9D2}]${TONE}?`;
const ROLE = "[\\u2695\\u2696\\u2708\\u{1F33E}\\u{1F373}\\u{1F37C}\\u{1F384}\\u{1F393}\\u{1F3A4}\\u{1F3A8}\\u{1F3EB}\\u{1F3ED}\\u{1F4BB}\\u{1F4BC}\\u{1F527}\\u{1F52C}\\u{1F680}\\u{1F692}\\u{1F9AF}\\u{1F9B0}-\\u{1F9B3}\\u{1F9BC}\\u{1F9BD}\\u{1FA70}]";
const WALKER = "[\\u{1F3C3}\\u{1F6B6}\\u{1F9CE}]";
const FACING = `${ZWJ}\\u27A1${VS}`;
const GENDER_BASE = "[\\u26F9\\u{1F3C3}\\u{1F3C4}\\u{1F3CA}-\\u{1F3CC}\\u{1F46E}\\u{1F46F}\\u{1F470}\\u{1F471}\\u{1F473}\\u{1F477}\\u{1F481}\\u{1F482}\\u{1F486}\\u{1F487}\\u{1F575}\\u{1F645}-\\u{1F647}\\u{1F64B}\\u{1F64D}\\u{1F64E}\\u{1F6A3}\\u{1F6B4}-\\u{1F6B6}\\u{1F926}\\u{1F935}\\u{1F937}-\\u{1F939}\\u{1F93C}-\\u{1F93E}\\u{1F9B8}\\u{1F9B9}\\u{1F9CD}-\\u{1F9CF}\\u{1F9D4}\\u{1F9D6}-\\u{1F9DF}]";
const GENDERED = `(?:${GENDER_BASE}${VS}${TONE}?${ZWJ}[\\u2640\\u2642]${VS}|${WALKER}${TONE}?${ZWJ}[\\u2640\\u2642]${VS}${FACING})`;
const WALKING = `${WALKER}${TONE}?${FACING}`;
const WITH_ROLE = `${PERSON}${ZWJ}(?:${ROLE}${VS}|[\\u{1F9AF}\\u{1F9BC}\\u{1F9BD}]${FACING})`;
const FAMILY = `${PERSON}(?:${ZWJ}${PERSON}){1,3}`;
const COUPLE = `${PERSON}${ZWJ}(?:\\u2764${VS}(?:${ZWJ}\\u{1F48B})?|\\u{1F91D})${ZWJ}${PERSON}`;
const HANDSHAKE = `\\u{1FAF1}${TONE}${ZWJ}\\u{1FAF2}${TONE}`;
const SPECIAL = [
  "\\u{1F3F3}\\uFE0F?\\u200D(?:\\u{1F308}|\\u26A7\\uFE0F?)",
  "\\u{1F3F4}\\u200D\\u2620\\uFE0F?",
  "\\u{1F441}\\uFE0F?\\u200D\\u{1F5E8}\\uFE0F?",
  "\\u{1F636}\\u200D\\u{1F32B}\\uFE0F?",
  "\\u{1F62E}\\u200D\\u{1F4A8}",
  "\\u{1F635}\\u200D\\u{1F4AB}",
  "\\u2764\\uFE0F?\\u200D(?:\\u{1F525}|\\u{1FA79})",
  "\\u{1F43B}\\u200D\\u2744\\uFE0F?",
  "(?:\\u{1F408}|\\u{1F426})\\u200D\\u2B1B",
  "\\u{1F426}\\u200D\\u{1F525}",
  "\\u{1F415}\\u200D\\u{1F9BA}",
  "\\u{1F34B}\\u200D\\u{1F7E9}",
  "\\u{1F344}\\u200D\\u{1F7EB}",
  "\\u{1F642}\\u200D[\\u2194\\u2195]\\uFE0F?",
  "\\u26D3\\uFE0F?\\u200D\\u{1F4A5}",
].join("|");
const ZWJ_SEQUENCE = `(?:${GENDERED}|${WALKING}|${WITH_ROLE}|${COUPLE}|${FAMILY}|${HANDSHAKE}|${SPECIAL})`;
const SEQUENCE = `(?:${ZWJ_SEQUENCE}|${ELEMENT})`;
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
