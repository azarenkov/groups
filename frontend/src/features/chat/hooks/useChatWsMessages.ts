import React from "react";
import type {
  Attachment,
  ChannelMessage,
  ForwardInfo,
  ReplyPreview,
  SystemMessageInfo,
} from "../../../shared/types/channel";
import { markReplyDeleted, refreshReplySnippet } from "../lib/replies";
import { useWsChannelMessages, useWsDmMessages } from "./useWebSocketMessages";
import { ws } from "../../../shared/ws/websocket";

export function useChatWsMessages(params: {
  channelId: string | null;
  dmFriendId: string | null;
  currentUserId?: string;
  setMessages: React.Dispatch<React.SetStateAction<ChannelMessage[]>>;
}) {
  const { channelId, dmFriendId, currentUserId, setMessages } = params;

  const channelMsgHandlers = React.useMemo(
    () => ({
      onSent: (m: {
        id: string;
        channelId: string;
        authorId: string;
        authorName?: string;
        content: string;
        occurredAt: string;
        attachments?: Attachment[];
        kind?: "user" | "system";
        system?: SystemMessageInfo;
        replyTo?: Omit<ReplyPreview, "deleted"> & { deleted?: boolean };
        forwardedFrom?: ForwardInfo | null;
      }) => {
        setMessages((prev) =>
          prev.some((x) => x.id === m.id)
            ? prev
            : [
                ...prev,
                {
                  id: m.id,
                  channelId: m.channelId,
                  authorId: m.authorId,
                  authorName: m.authorName,
                  content: m.content,
                  attachments: m.attachments,
                  kind: m.kind,
                  system: m.system,
                  replyTo: m.replyTo ? { ...m.replyTo, deleted: m.replyTo.deleted ?? false } : undefined,
                  forwardedFrom: m.forwardedFrom,
                  createdAt: m.occurredAt,
                  editedAt: null,
                },
              ],
        );
      },
      onEdited: (m: {
        id: string;
        channelId: string;
        newContent: string;
        occurredAt: string;
        attachments?: Attachment[];
      }) => {
        setMessages((prev) =>
          refreshReplySnippet(
            prev.map((x) =>
              x.id === m.id
                ? {
                    ...x,
                    content: m.newContent,
                    editedAt: m.occurredAt,
                    ...(m.attachments ? { attachments: m.attachments } : {}),
                  }
                : x,
            ),
            m.id,
            m.newContent,
            m.attachments ? m.attachments.length > 0 : undefined,
          ),
        );
      },
      onDeleted: (m: { id: string }) => {
        setMessages((prev) => markReplyDeleted(prev.filter((x) => x.id !== m.id), m.id));
      },
    }),
    [setMessages],
  );
  useWsChannelMessages(channelId, channelMsgHandlers);

  const dmMsgHandlers = React.useMemo(
    () => ({
      onSent: (m: {
        id: string;
        senderId: string;
        senderName?: string;
        recipientId: string;
        content: string;
        occurredAt: string;
        inviteCommunityId?: string;
        inviteCommunityName?: string;
        callStatus?: string;
        callDurationSeconds?: number;
        attachments?: Attachment[];
        replyTo?: (Omit<ReplyPreview, "deleted"> & { deleted?: boolean }) | null;
        forwardedFrom?: ForwardInfo | null;
      }) => {
        setMessages((prev) =>
          prev.some((x) => x.id === m.id)
            ? prev
            : [
                ...prev,
                {
                  id: m.id,
                  channelId: m.senderId,
                  authorId: m.senderId,
                  authorName: m.senderName,
                  content: m.content,
                  inviteCommunityId: m.inviteCommunityId,
                  inviteCommunityName: m.inviteCommunityName,
                  callStatus: m.callStatus,
                  callDurationSeconds: m.callDurationSeconds,
                  attachments: m.attachments,
                  replyTo: m.replyTo ? { ...m.replyTo, deleted: m.replyTo.deleted ?? false } : undefined,
                  forwardedFrom: m.forwardedFrom,
                  createdAt: m.occurredAt,
                  editedAt: null,
                },
              ],
        );
      },
      onEdited: (m: {
        id: string;
        senderId: string;
        newContent: string;
        occurredAt: string;
        attachments?: Attachment[];
      }) => {
        setMessages((prev) =>
          refreshReplySnippet(
            prev.map((x) =>
              x.id === m.id
                ? {
                    ...x,
                    content: m.newContent,
                    editedAt: m.occurredAt,
                    ...(m.attachments ? { attachments: m.attachments } : {}),
                  }
                : x,
            ),
            m.id,
            m.newContent,
            m.attachments ? m.attachments.length > 0 : undefined,
          ),
        );
      },
      onDeleted: (m: { id: string }) => {
        setMessages((prev) => markReplyDeleted(prev.filter((x) => x.id !== m.id), m.id));
      },
    }),
    [setMessages],
  );
  useWsDmMessages(dmFriendId, currentUserId, dmMsgHandlers);

  React.useEffect(
    () =>
      ws.on((env) => {
        if (env.type !== "voice.listened") return;
        const id = (env.data as { messageId?: unknown }).messageId;
        if (typeof id !== "string") return;
        setMessages((prev) =>
          prev.some((m) => m.id === id && !m.voiceListened)
            ? prev.map((m) => (m.id === id ? { ...m, voiceListened: true } : m))
            : prev,
        );
      }),
    [setMessages],
  );

  React.useEffect(
    () =>
      ws.on((env) => {
        if (env.type !== "message.hidden") return;
        const id = (env.data as { messageId?: unknown }).messageId;
        if (typeof id !== "string") return;
        setMessages((prev) => markReplyDeleted(prev.filter((m) => m.id !== id), id));
      }),
    [setMessages],
  );
}
