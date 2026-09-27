import type { ChannelMessage, ReplyPreview } from "../../../shared/types/channel";
import { voiceAttachmentOf } from "./voice";

export const REPLY_SNIPPET_CHARS = 200;

export function replySnippet(content: string): string {
  const flat = content.split(/\s+/).filter(Boolean).join(" ");
  const chars = Array.from(flat);
  if (chars.length <= REPLY_SNIPPET_CHARS) return flat;
  return `${chars.slice(0, REPLY_SNIPPET_CHARS).join("")}…`;
}

export function previewOf(msg: ChannelMessage, authorName?: string): ReplyPreview {
  return {
    id: msg.id,
    authorId: msg.authorId,
    authorName: authorName ?? msg.authorName,
    content: replySnippet(msg.content),
    hasAttachments: (msg.attachments?.length ?? 0) > 0,
    voiceDurationMs: voiceAttachmentOf(msg)?.voice?.durationMs ?? null,
    deleted: false,
  };
}

export function markReplyDeleted(messages: ChannelMessage[], id: string): ChannelMessage[] {
  if (!messages.some((m) => m.replyTo?.id === id)) return messages;
  return messages.map((m) =>
    m.replyTo?.id === id
      ? { ...m, replyTo: { id, content: "", hasAttachments: false, deleted: true } }
      : m,
  );
}

export function refreshReplySnippet(
  messages: ChannelMessage[],
  id: string,
  content: string,
  hasAttachments?: boolean,
): ChannelMessage[] {
  if (!messages.some((m) => m.replyTo?.id === id)) return messages;
  const snippet = replySnippet(content);
  return messages.map((m) =>
    m.replyTo?.id === id && !m.replyTo.deleted
      ? {
          ...m,
          replyTo: {
            ...m.replyTo,
            content: snippet,
            hasAttachments: hasAttachments ?? m.replyTo.hasAttachments,
          },
        }
      : m,
  );
}
