import { http } from "../../../shared/api/http";
import {
  ListChannelsResponse,
  ListChannelParticipantsResponse,
  CreateChannelRequest,
  CreateChannelResponse,
  JoinVoiceChannelRequest,
  JoinVoiceChannelResponse,
  ChannelMessage,
  ListChannelMessagesResponse,
  SearchChannelMessagesResponse,
} from "../../../shared/types/channel";

class ChannelAPI {
  private client = http;

  async listByCommunity(
    communityId: string,
    page: number = 1,
    perPage: number = 50,
  ): Promise<ListChannelsResponse> {
    const response = await this.client.get(
      `/v1/communities/${communityId}/channels`,
      { params: { page, per_page: perPage } },
    );
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async listParticipants(
    communityId: string,
    channelId: string,
    page: number = 1,
    perPage: number = 50,
  ): Promise<ListChannelParticipantsResponse> {
    const response = await this.client.get(
      `/v1/communities/${communityId}/channels/${channelId}/participants`,
      { params: { page, per_page: perPage } },
    );
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async create(
    communityId: string,
    data: CreateChannelRequest,
  ): Promise<CreateChannelResponse> {
    const response = await this.client.post(
      `/v1/communities/${communityId}/channels`,
      data,
    );
    return response.data.data;
  }

  async delete(communityId: string, channelId: string): Promise<void> {
    await this.client.delete(
      `/v1/communities/${communityId}/channels/${channelId}`,
    );
  }

  async joinVoice(
    communityId: string,
    channelId: string,
    data: JoinVoiceChannelRequest = {},
  ): Promise<JoinVoiceChannelResponse> {
    const response = await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/join_voice`,
      data,
    );
    return response.data.data;
  }

  async mute(
    communityId: string,
    channelId: string,
    targetUserId: string,
  ): Promise<void> {
    await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/mute`,
      { targetUserId },
    );
  }

  async unmute(
    communityId: string,
    channelId: string,
    targetUserId: string,
  ): Promise<void> {
    await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/unmute`,
      { targetUserId },
    );
  }

  async disconnect(
    communityId: string,
    channelId: string,
    targetUserId: string,
  ): Promise<void> {
    await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/disconnect`,
      { targetUserId },
    );
  }

  async deafen(
    communityId: string,
    channelId: string,
    targetUserId: string,
  ): Promise<void> {
    await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/deafen`,
      { targetUserId },
    );
  }

  async undeafen(
    communityId: string,
    channelId: string,
    targetUserId: string,
  ): Promise<void> {
    await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/undeafen`,
      { targetUserId },
    );
  }

  async listMessages(
    communityId: string,
    channelId: string,
    opts: { before?: string; after?: string; limit?: number } = {},
  ): Promise<ListChannelMessagesResponse> {
    const params: Record<string, string | number> = {};
    if (opts.before) params.before = opts.before;
    if (opts.after) params.after = opts.after;
    if (opts.limit !== undefined) params.limit = opts.limit;
    const response = await this.client.get(
      `/v1/communities/${communityId}/channels/${channelId}/messages`,
      { params },
    );
    return response.data.data;
  }

  async searchMessages(
    communityId: string,
    channelId: string,
    query: string,
    page: number = 1,
    perPage: number = 25,
  ): Promise<SearchChannelMessagesResponse> {
    const response = await this.client.get(
      `/v1/communities/${communityId}/channels/${channelId}/messages/search`,
      { params: { q: query, page, per_page: perPage } },
    );
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async sendMessage(
    communityId: string,
    channelId: string,
    content: string,
    attachmentIds?: string[],
    replyToMessageId?: string,
    stickerId?: string,
  ): Promise<ChannelMessage> {
    const response = await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/messages`,
      { content, attachmentIds, replyToMessageId, stickerId },
    );
    return response.data.data;
  }

  async getMessage(
    communityId: string,
    channelId: string,
    messageId: string,
  ): Promise<ChannelMessage> {
    const response = await this.client.get(
      `/v1/communities/${communityId}/channels/${channelId}/messages/${messageId}`,
    );
    return response.data.data;
  }

  async editMessage(
    communityId: string,
    channelId: string,
    messageId: string,
    content: string,
    addAttachmentIds?: string[],
    removeAttachmentIds?: string[],
  ): Promise<void> {
    await this.client.patch(
      `/v1/communities/${communityId}/channels/${channelId}/messages/${messageId}`,
      {
        content,
        addAttachmentIds: addAttachmentIds ?? [],
        removeAttachmentIds: removeAttachmentIds ?? [],
      },
    );
  }

  async deleteMessage(
    communityId: string,
    channelId: string,
    messageId: string,
  ): Promise<void> {
    await this.client.delete(
      `/v1/communities/${communityId}/channels/${channelId}/messages/${messageId}`,
    );
  }

  async markVoiceListened(communityId: string, channelId: string, messageId: string): Promise<void> {
    await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/messages/${messageId}/listened`,
    );
  }

  async hideMessage(communityId: string, channelId: string, messageId: string): Promise<void> {
    await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/messages/${messageId}/hide`,
    );
  }

  async joinText(
    communityId: string,
    channelId: string,
    password?: string,
  ): Promise<void> {
    const body: { password?: string } = {};
    if (password !== undefined) body.password = password;
    await this.client.post(
      `/v1/communities/${communityId}/channels/${channelId}/join`,
      body,
    );
  }
}

export const channelAPI = new ChannelAPI();
