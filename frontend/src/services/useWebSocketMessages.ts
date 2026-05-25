import { useEffect } from "react";

import { ws, WsEnvelope } from "./websocket";

export interface ChannelMessageHandlers {
  onSent?: (m: {
    id: string;
    channelId: string;
    authorId: string;
    authorName?: string;
    content: string;
    occurredAt: string;
  }) => void;
  onEdited?: (m: {
    id: string;
    channelId: string;
    newContent: string;
    occurredAt: string;
  }) => void;
  onDeleted?: (m: {
    id: string;
    channelId: string;
    deletedBy: string;
    occurredAt: string;
  }) => void;
}

export function useWsChannelMessages(
  channelId: string | null,
  handlers: ChannelMessageHandlers,
) {
  useEffect(() => {
    if (!channelId) return;
    const off = ws.on((env: WsEnvelope) => {
      const d = env.data as Record<string, unknown>;
      if (typeof d.channelId !== "string" || d.channelId !== channelId) return;
      if (env.type === "message.sent" && handlers.onSent) {
        handlers.onSent(d as Parameters<NonNullable<typeof handlers.onSent>>[0]);
      } else if (env.type === "message.edited" && handlers.onEdited) {
        handlers.onEdited(
          d as Parameters<NonNullable<typeof handlers.onEdited>>[0],
        );
      } else if (env.type === "message.deleted" && handlers.onDeleted) {
        handlers.onDeleted(
          d as Parameters<NonNullable<typeof handlers.onDeleted>>[0],
        );
      }
    });
    return off;
  }, [channelId, handlers.onSent, handlers.onEdited, handlers.onDeleted]);
}

export interface DmMessageHandlers {
  onSent?: (m: {
    id: string;
    senderId: string;
    senderName?: string;
    recipientId: string;
    content: string;
    occurredAt: string;
    inviteServerId?: string;
    inviteServerName?: string;
  }) => void;
  onEdited?: (m: {
    id: string;
    senderId: string;
    newContent: string;
    occurredAt: string;
  }) => void;
  onDeleted?: (m: {
    id: string;
    senderId: string;
    recipientId: string;
    occurredAt: string;
  }) => void;
}

export function useWsDmMessages(
  otherUserId: string | null,
  selfUserId: string | undefined,
  handlers: DmMessageHandlers,
) {
  useEffect(() => {
    if (!otherUserId || !selfUserId) return;
    const off = ws.on((env: WsEnvelope) => {
      const d = env.data as Record<string, unknown>;
      const sender = typeof d.senderId === "string" ? d.senderId : null;
      const recipient = typeof d.recipientId === "string" ? d.recipientId : null;
      const involvesPair =
        (sender === selfUserId && recipient === otherUserId) ||
        (sender === otherUserId && recipient === selfUserId) ||
        (env.type === "dm_message.edited" &&
          (sender === selfUserId || sender === otherUserId));
      if (!involvesPair) return;
      if (env.type === "dm_message.sent" && handlers.onSent) {
        handlers.onSent(d as Parameters<NonNullable<typeof handlers.onSent>>[0]);
      } else if (env.type === "dm_message.edited" && handlers.onEdited) {
        handlers.onEdited(
          d as Parameters<NonNullable<typeof handlers.onEdited>>[0],
        );
      } else if (env.type === "dm_message.deleted" && handlers.onDeleted) {
        handlers.onDeleted(
          d as Parameters<NonNullable<typeof handlers.onDeleted>>[0],
        );
      }
    });
    return off;
  }, [
    otherUserId,
    selfUserId,
    handlers.onSent,
    handlers.onEdited,
    handlers.onDeleted,
  ]);
}
