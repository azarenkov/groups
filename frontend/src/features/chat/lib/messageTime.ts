import type { ChannelMessage } from "../../../shared/types/channel";

const MINUTE_MS = 60 * 1000;

const showsTime = (m: ChannelMessage) => !m.system && !m.callStatus;

export function sharesTimeWithNext(m: ChannelMessage, next: ChannelMessage | undefined): boolean {
  if (!next || !showsTime(m) || !showsTime(next)) return false;
  if (m.authorId !== next.authorId) return false;
  const minute = Math.floor(new Date(m.createdAt).getTime() / MINUTE_MS);
  const nextMinute = Math.floor(new Date(next.createdAt).getTime() / MINUTE_MS);
  return minute === nextMinute;
}
