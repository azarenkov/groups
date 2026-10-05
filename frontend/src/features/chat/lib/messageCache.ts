import type { ChannelMessage } from "../../../shared/types/channel";
import { ws } from "../../../shared/ws/websocket";
import { useAuthStore } from "../../auth/store/authStore";
import { PAGE_LIMIT } from "../constants";
import { markReplyDeleted, refreshReplySnippet } from "./replies";

export const MESSAGE_CACHE_LIMIT = 20;

export interface ScrollMemory {
  atBottom: boolean;
  anchorId: string | null;
  offset: number;
}

export interface CachedConversation {
  messages: ChannelMessage[];
  hasMore: boolean;
  scroll: ScrollMemory;
}

const entries = new Map<string, CachedConversation>();

export const hasCachedConversation = (key: string): boolean => entries.has(key);

export function readConversation(key: string): CachedConversation | undefined {
  const entry = entries.get(key);
  if (!entry) return undefined;
  entries.delete(key);
  entries.set(key, entry);
  return entry;
}

export function saveConversation(key: string, entry: CachedConversation): void {
  entries.delete(key);
  entries.set(key, entry);
  while (entries.size > MESSAGE_CACHE_LIMIT) {
    const oldest = entries.keys().next().value;
    if (oldest === undefined) break;
    entries.delete(oldest);
  }
}

export function dropConversation(key: string): void {
  entries.delete(key);
}

export function clearMessageCache(): void {
  entries.clear();
}

export function patchConversation(key: string, patch: (messages: ChannelMessage[]) => ChannelMessage[]): void {
  const entry = entries.get(key);
  if (!entry) return;
  const messages = patch(entry.messages);
  if (messages !== entry.messages) entries.set(key, { ...entry, messages });
}

export function trimCachedConversations(keep = PAGE_LIMIT): void {
  for (const [key, entry] of entries) {
    if (entry.messages.length <= keep) continue;
    entries.set(key, {
      messages: entry.messages.slice(-keep),
      hasMore: true,
      scroll: { atBottom: true, anchorId: null, offset: 0 },
    });
  }
}

export function reconcileLatestPage(
  current: ChannelMessage[],
  latest: ChannelMessage[],
  fullPage: boolean,
): { messages: ChannelMessage[]; gap: boolean } {
  const ids = new Set(latest.map((m) => m.id));
  const newest = latest[latest.length - 1]?.createdAt;
  const live = current.filter((m) => !ids.has(m.id) && (!newest || m.createdAt > newest));
  if (!fullPage || latest.length === 0) {
    return { messages: [...latest, ...live], gap: false };
  }
  const cutoff = latest[0].createdAt;
  const reachesWindow = current.some((m) => m.createdAt >= cutoff);
  if (!reachesWindow) return { messages: [...latest, ...live], gap: true };
  const older = current.filter((m) => m.createdAt < cutoff && !ids.has(m.id));
  return { messages: [...older, ...latest, ...live], gap: false };
}

const dropMessage = (key: string, id: string) =>
  patchConversation(key, (messages) =>
    messages.some((m) => m.id === id) ? markReplyDeleted(messages.filter((m) => m.id !== id), id) : messages,
  );

let installed = false;

export function installMessageCacheSync(): void {
  if (installed) return;
  installed = true;
  ws.onOpen(() => trimCachedConversations());
  ws.on((env) => {
    const d = env.data as Record<string, unknown>;
    const str = (v: unknown) => (typeof v === "string" ? v : undefined);
    const dmKey = () => {
      const self = useAuthStore.getState().user?.id;
      const sender = str(d.senderId);
      const recipient = str(d.recipientId);
      if (!self || !sender || !recipient) return undefined;
      return sender === self ? recipient : sender;
    };
    const edit = (key: string | undefined) => {
      const id = str(d.id);
      const content = str(d.newContent);
      if (!key || !id || content === undefined) return;
      const attachments = Array.isArray(d.attachments) ? (d.attachments as ChannelMessage["attachments"]) : undefined;
      patchConversation(key, (messages) =>
        messages.some((m) => m.id === id)
          ? refreshReplySnippet(
              messages.map((m) =>
                m.id === id
                  ? { ...m, content, editedAt: str(d.occurredAt) ?? m.editedAt, ...(attachments ? { attachments } : {}) }
                  : m,
              ),
              id,
              content,
              attachments ? attachments.length > 0 : undefined,
            )
          : messages,
      );
    };
    switch (env.type) {
      case "message.deleted": {
        const key = str(d.channelId);
        const id = str(d.id);
        if (key && id) dropMessage(key, id);
        return;
      }
      case "dm_message.deleted": {
        const key = dmKey();
        const id = str(d.id);
        if (key && id) dropMessage(key, id);
        return;
      }
      case "message.edited":
        edit(str(d.channelId));
        return;
      case "dm_message.edited":
        edit(dmKey());
        return;
      case "message.hidden": {
        const id = str(d.messageId);
        if (!id) return;
        for (const key of Array.from(entries.keys())) dropMessage(key, id);
        return;
      }
      case "voice.listened": {
        const id = str(d.messageId);
        if (!id) return;
        for (const key of Array.from(entries.keys())) {
          patchConversation(key, (messages) =>
            messages.some((m) => m.id === id && !m.voiceListened)
              ? messages.map((m) => (m.id === id ? { ...m, voiceListened: true } : m))
              : messages,
          );
        }
        return;
      }
      default:
        return;
    }
  });
}
