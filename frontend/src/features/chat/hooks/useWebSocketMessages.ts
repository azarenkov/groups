import { useEffect, useRef } from "react";

import { ws, WsEnvelope } from "../../../shared/ws/websocket";

export interface ChannelMessageHandlers {
  onSent?: (m: {
    id: string;
    channelId: string;
    authorId: string;
    authorName?: string;
    content: string;
    occurredAt: string;
    attachments?: {
      id: string;
      fileName: string;
      contentType: string;
      sizeBytes: number;
    }[];
  }) => void;
  onEdited?: (m: {
    id: string;
    channelId: string;
    newContent: string;
    occurredAt: string;
    attachments?: {
      id: string;
      fileName: string;
      contentType: string;
      sizeBytes: number;
    }[];
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
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;
  useEffect(() => {
    if (!channelId) return;
    const off = ws.on((env: WsEnvelope) => {
      const d = env.data as Record<string, unknown>;
      if (typeof d.channelId !== "string" || d.channelId !== channelId) return;
      if (env.type === "message.sent" && handlersRef.current.onSent) {
        handlersRef.current.onSent(d as Parameters<NonNullable<typeof handlersRef.current.onSent>>[0]);
      } else if (env.type === "message.edited" && handlersRef.current.onEdited) {
        handlersRef.current.onEdited(
          d as Parameters<NonNullable<typeof handlersRef.current.onEdited>>[0],
        );
      } else if (env.type === "message.deleted" && handlersRef.current.onDeleted) {
        handlersRef.current.onDeleted(
          d as Parameters<NonNullable<typeof handlersRef.current.onDeleted>>[0],
        );
      }
    });
    return off;
  }, [channelId]);
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
    callStatus?: string;
    callDurationSeconds?: number;
    attachments?: {
      id: string;
      fileName: string;
      contentType: string;
      sizeBytes: number;
    }[];
  }) => void;
  onEdited?: (m: {
    id: string;
    senderId: string;
    newContent: string;
    occurredAt: string;
    attachments?: {
      id: string;
      fileName: string;
      contentType: string;
      sizeBytes: number;
    }[];
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
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;
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
      if (env.type === "dm_message.sent" && handlersRef.current.onSent) {
        handlersRef.current.onSent(d as Parameters<NonNullable<typeof handlersRef.current.onSent>>[0]);
      } else if (env.type === "dm_message.edited" && handlersRef.current.onEdited) {
        handlersRef.current.onEdited(
          d as Parameters<NonNullable<typeof handlersRef.current.onEdited>>[0],
        );
      } else if (env.type === "dm_message.deleted" && handlersRef.current.onDeleted) {
        handlersRef.current.onDeleted(
          d as Parameters<NonNullable<typeof handlersRef.current.onDeleted>>[0],
        );
      }
    });
    return off;
  }, [otherUserId, selfUserId]);
}
