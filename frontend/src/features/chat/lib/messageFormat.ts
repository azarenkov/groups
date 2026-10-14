import i18n from "../../../i18n";
import type { ChannelMessage } from "../../../shared/types/channel";

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

export type MessageCursor = Pick<ChannelMessage, "id" | "createdAt">;

export const cursorAfter = (a: MessageCursor, b: MessageCursor): boolean =>
  a.createdAt === b.createdAt ? a.id.toLowerCase() > b.id.toLowerCase() : a.createdAt > b.createdAt;

export const reconcileSyncedWindow = (
  synced: ChannelMessage[],
  prev: ChannelMessage[],
  reachedStart: boolean,
  knownBefore: ReadonlySet<string>,
): ChannelMessage[] => {
  const ids = new Set(synced.map((m) => m.id));
  const from = reachedStart ? undefined : synced[0]?.createdAt;
  const kept = prev.filter(
    (m) => ids.has(m.id) || !knownBefore.has(m.id) || (from !== undefined && m.createdAt < from),
  );
  return mergeWithLive(synced, kept);
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
  const total = Math.max(0, Math.floor(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const ss = String(s).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
};
