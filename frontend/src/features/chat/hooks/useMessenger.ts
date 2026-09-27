import React from "react";
import { channelAPI } from "../../communities/api/channel";
import { dmAPI } from "../../dm/api/dm";
import type { ChannelMessage, PaginationMeta } from "../../../shared/types/channel";
import type { ChannelMeta, DmMeta } from "../types";

export interface Messenger {
  listMessages: (opts: {
    before?: string;
    after?: string;
    limit?: number;
  }) => Promise<{ messages: ChannelMessage[] }>;
  sendMessage: (
    content: string,
    attachmentIds?: string[],
    replyToMessageId?: string,
  ) => Promise<ChannelMessage>;
  getMessage: (messageId: string) => Promise<ChannelMessage>;
  editMessage: (
    messageId: string,
    content: string,
    addAttachmentIds?: string[],
    removeAttachmentIds?: string[],
  ) => Promise<void>;
  deleteMessage: (messageId: string) => Promise<void>;
  hideMessage: (messageId: string) => Promise<void>;
  searchMessages: (
    query: string,
    page: number,
    perPage: number,
  ) => Promise<{ messages: ChannelMessage[]; pagination?: PaginationMeta }>;
}

export function useMessenger(params: {
  channel?: ChannelMeta;
  communityId?: string;
  dm?: DmMeta;
  isDm: boolean;
}): Messenger | null {
  const { channel, communityId, dm, isDm } = params;
  return React.useMemo<Messenger | null>(() => {
    if (isDm && dm) {
      return {
        listMessages: (opts) => dmAPI.listMessages(dm.friendId, opts),
        sendMessage: (content, attachmentIds, replyToMessageId) =>
          dmAPI.sendMessage(dm.friendId, content, attachmentIds, replyToMessageId),
        getMessage: (messageId) => dmAPI.getMessage(dm.friendId, messageId),
        editMessage: (messageId, content, addAttachmentIds, removeAttachmentIds) =>
          dmAPI.editMessage(
            messageId,
            content,
            addAttachmentIds,
            removeAttachmentIds,
          ),
        deleteMessage: (messageId) => dmAPI.deleteMessage(messageId),
        hideMessage: (messageId) => dmAPI.hideMessage(dm.friendId, messageId),
        searchMessages: (query, page, perPage) =>
          dmAPI.searchMessages(dm.friendId, query, page, perPage),
      };
    }
    if (channel && communityId) {
      return {
        listMessages: (opts) =>
          channelAPI.listMessages(communityId, channel.id, opts),
        sendMessage: (content, attachmentIds, replyToMessageId) =>
          channelAPI.sendMessage(communityId, channel.id, content, attachmentIds, replyToMessageId),
        getMessage: (messageId) =>
          channelAPI.getMessage(communityId, channel.id, messageId),
        editMessage: (messageId, content, addAttachmentIds, removeAttachmentIds) =>
          channelAPI.editMessage(
            communityId,
            channel.id,
            messageId,
            content,
            addAttachmentIds,
            removeAttachmentIds,
          ),
        deleteMessage: (messageId) =>
          channelAPI.deleteMessage(communityId, channel.id, messageId),
        hideMessage: (messageId) =>
          channelAPI.hideMessage(communityId, channel.id, messageId),
        searchMessages: (query, page, perPage) =>
          channelAPI.searchMessages(communityId, channel.id, query, page, perPage),
      };
    }
    return null;
  }, [isDm, dm, channel, communityId]);
}
