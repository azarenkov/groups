import React from "react";
import type {
  Attachment,
  ChannelMessage,
  SystemMessageInfo,
} from "../../../shared/types/channel";
import { useWsChannelMessages, useWsDmMessages } from "./useWebSocketMessages";

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
        );
      },
      onDeleted: (m: { id: string }) => {
        setMessages((prev) => prev.filter((x) => x.id !== m.id));
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
        );
      },
      onDeleted: (m: { id: string }) => {
        setMessages((prev) => prev.filter((x) => x.id !== m.id));
      },
    }),
    [setMessages],
  );
  useWsDmMessages(dmFriendId, currentUserId, dmMsgHandlers);
}
