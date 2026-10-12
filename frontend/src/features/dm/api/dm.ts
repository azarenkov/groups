import { http } from "../../../shared/api/http";

import { Attachment, ChannelMessage, ForwardInfo, ReplyPreview, StickerSnapshot } from "../../../shared/types/channel";

interface DmRow {
  id: string;
  authorId: string;
  authorName?: string;
  recipientId: string;
  content: string;
  inviteCommunityId?: string;
  inviteCommunityName?: string;
  callStatus?: string;
  callDurationSeconds?: number;
  attachments?: Attachment[];
  replyTo?: ReplyPreview;
  forwardedFrom?: ForwardInfo | null;
  sticker?: StickerSnapshot | null;
  voiceListened?: boolean;
  createdAt: string;
  editedAt: string | null;
}

const toChannelMessage = (r: DmRow): ChannelMessage => ({
  id: r.id,
  channelId: "",
  authorId: r.authorId,
  authorName: r.authorName,
  content: r.content,
  inviteCommunityId: r.inviteCommunityId,
  inviteCommunityName: r.inviteCommunityName,
  callStatus: r.callStatus,
  callDurationSeconds: r.callDurationSeconds,
  attachments: r.attachments,
  replyTo: r.replyTo,
  forwardedFrom: r.forwardedFrom,
  sticker: r.sticker,
  voiceListened: r.voiceListened,
  createdAt: r.createdAt,
  editedAt: r.editedAt,
});

class DmAPI {
  private client = http;

  async listMessages(
    friendId: string,
    opts: { before?: string; after?: string; limit?: number } = {},
  ): Promise<{ messages: ChannelMessage[]; partition?: string; seq?: number }> {
    const params: Record<string, string | number> = {};
    if (opts.before) params.before = opts.before;
    if (opts.after) params.after = opts.after;
    if (opts.limit !== undefined) params.limit = opts.limit;
    const response = await this.client.get(
      `/v1/users/me/dms/${friendId}/messages`,
      { params },
    );
    const { partition, seq } = response.data.data as { partition?: unknown; seq?: unknown };
    return {
      messages: (response.data.data.messages as DmRow[]).map(toChannelMessage),
      partition: typeof partition === "string" ? partition : undefined,
      seq: typeof seq === "number" ? seq : undefined,
    };
  }

  async listPreviews(opts: { cursor?: string | null; limit?: number } = {}): Promise<{
    previews: {
      messageId: string;
      peerId: string;
      peerName: string;
      senderId: string;
      content: string;
      callStatus: string | null;
      inviteCommunityId: string | null;
      hasAttachments: boolean;
      voiceDurationMs?: number | null;
      videoNoteDurationMs?: number | null;
      stickerEmoji?: string | null;
      createdAt: string;
    }[];
    nextCursor: string | null;
  }> {
    const response = await this.client.get(`/v1/users/me/dms/previews`, {
      params: { cursor: opts.cursor ?? undefined, limit: opts.limit },
    });
    return response.data.data;
  }

  async sendMessage(
    friendId: string,
    content: string,
    attachmentIds?: string[],
    replyToMessageId?: string,
    stickerId?: string,
  ): Promise<ChannelMessage> {
    const response = await this.client.post(
      `/v1/users/me/dms/${friendId}/messages`,
      { content, attachmentIds, replyToMessageId, stickerId },
    );
    return toChannelMessage(response.data.data);
  }

  async getMessage(friendId: string, messageId: string): Promise<ChannelMessage> {
    const response = await this.client.get(
      `/v1/users/me/dms/${friendId}/messages/${messageId}`,
    );
    return toChannelMessage(response.data.data);
  }

  async sendCommunityInvite(
    friendId: string,
    communityId: string,
  ): Promise<ChannelMessage> {
    const response = await this.client.post(
      `/v1/users/me/dms/${friendId}/community-invite`,
      { communityId },
    );
    return toChannelMessage(response.data.data);
  }

  async acceptCommunityInvite(messageId: string): Promise<{ communityId: string }> {
    const response = await this.client.post(
      `/v1/users/me/dms/messages/${messageId}/accept-invite`,
    );
    return { communityId: response.data.data.communityId };
  }

  async declineCommunityInvite(messageId: string): Promise<void> {
    await this.client.post(
      `/v1/users/me/dms/messages/${messageId}/decline-invite`,
    );
  }

  async editMessage(
    messageId: string,
    content: string,
    addAttachmentIds?: string[],
    removeAttachmentIds?: string[],
  ): Promise<void> {
    await this.client.patch(`/v1/users/me/dms/messages/${messageId}`, {
      content,
      addAttachmentIds: addAttachmentIds ?? [],
      removeAttachmentIds: removeAttachmentIds ?? [],
    });
  }

  async deleteMessage(messageId: string): Promise<void> {
    await this.client.delete(`/v1/users/me/dms/messages/${messageId}`);
  }

  async markVoiceListened(friendId: string, messageId: string): Promise<void> {
    await this.client.post(`/v1/users/me/dms/${friendId}/messages/${messageId}/listened`);
  }

  async hideMessage(friendId: string, messageId: string): Promise<void> {
    await this.client.post(`/v1/users/me/dms/${friendId}/messages/${messageId}/hide`);
  }

  async clearConversation(friendId: string, forEveryone: boolean): Promise<{ clearedAt: string }> {
    const response = await this.client.post(`/v1/users/me/dms/${friendId}/clear`, { forEveryone });
    return response.data.data;
  }

  async searchMessages(
    friendId: string,
    query: string,
    page: number = 1,
    perPage: number = 25,
  ): Promise<{
    messages: ChannelMessage[];
    pagination?: {
      total: number;
      page: number;
      perPage: number;
      totalPages: number;
    };
  }> {
    const response = await this.client.get(
      `/v1/users/me/dms/${friendId}/messages/search`,
      { params: { q: query, page, per_page: perPage } },
    );
    return {
      messages: (response.data.data.messages as DmRow[]).map(toChannelMessage),
      pagination: response.data.meta?.pagination,
    };
  }
}

export const dmAPI = new DmAPI();
