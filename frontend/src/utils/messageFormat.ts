import i18n from "../i18n";
import type { ChannelMessage } from "../types/channel";

export const mergeWithLive = (
  history: ChannelMessage[],
  prev: ChannelMessage[],
): ChannelMessage[] => {
  const ids = new Set(history.map((m) => m.id));
  const extras = prev.filter((m) => !ids.has(m.id));
  if (extras.length === 0) return history;
  return [...history, ...extras].sort((a, b) =>
    a.createdAt < b.createdAt ? -1 : a.createdAt > b.createdAt ? 1 : 0,
  );
};

export const sameDay = (a: Date, b: Date): boolean =>
  a.getFullYear() === b.getFullYear() &&
  a.getMonth() === b.getMonth() &&
  a.getDate() === b.getDate();

export const formatDayDivider = (date: Date): string => {
  const today = new Date();
  const yesterday = new Date();
  yesterday.setDate(today.getDate() - 1);
  if (sameDay(date, today)) return i18n.t("today", { ns: "chat", defaultValue: "Today" });
  if (sameDay(date, yesterday)) return i18n.t("yesterday", { ns: "chat", defaultValue: "Yesterday" });
  return date.toLocaleDateString(i18n.language, {
    month: "long",
    day: "numeric",
    year: "numeric",
  });
};

export const splitForSend = (text: string, maxLen: number): string[] => {
  const chunks: string[] = [];
  let remaining = text;
  while (remaining.length > maxLen) {
    const slice = remaining.slice(0, maxLen);
    const nl = slice.lastIndexOf("\n");
    let cut = maxLen;
    if (nl >= maxLen - 200 && nl > 0) {
      cut = nl + 1;
    } else {
      const sp = slice.lastIndexOf(" ");
      if (sp >= maxLen - 100 && sp > 0) cut = sp + 1;
    }
    const piece = remaining.slice(0, cut).trimEnd();
    if (piece.length > 0) chunks.push(piece);
    remaining = remaining.slice(cut).trimStart();
  }
  const tail = remaining.trim();
  if (tail.length > 0) chunks.push(tail);
  return chunks;
};

export const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

export const formatCallDuration = (seconds: number): string => {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
};
