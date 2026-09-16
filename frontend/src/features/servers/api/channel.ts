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

  async listByServer(
    serverId: string,
    page: number = 1,
    perPage: number = 50,
  ): Promise<ListChannelsResponse> {
    const response = await this.client.get(
      `/v1/servers/${serverId}/channels`,
      { params: { page, per_page: perPage } },
    );
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async listParticipants(
    serverId: string,
    channelId: string,
    page: number = 1,
    perPage: number = 50,
  ): Promise<ListChannelParticipantsResponse> {
    const response = await this.client.get(
      `/v1/servers/${serverId}/channels/${channelId}/participants`,
      { params: { page, per_page: perPage } },
    );
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async create(
    serverId: string,
    data: CreateChannelRequest,
  ): Promise<CreateChannelResponse> {
    const response = await this.client.post(
      `/v1/servers/${serverId}/channels`,
      data,
    );
    return response.data.data;
  }

  async delete(serverId: string, channelId: string): Promise<void> {
    await this.client.delete(
      `/v1/servers/${serverId}/channels/${channelId}`,
    );
  }

  async joinVoice(
    serverId: string,
    channelId: string,
    data: JoinVoiceChannelRequest = {},
  ): Promise<JoinVoiceChannelResponse> {
    const response = await this.client.post(
      `/v1/servers/${serverId}/channels/${channelId}/join_voice`,
      data,
    );
    return response.data.data;
  }

  async mute(
    serverId: string,
    channelId: string,
    targetUserId: string,
  ): Promise<void> {
    await this.client.post(
      `/v1/servers/${serverId}/channels/${channelId}/mute`,
      { targetUserId },
    );
  }

  async unmute(
    serverId: string,
    channelId: string,
    targetUserId: string,
  ): Promise<void> {
    await this.client.post(
      `/v1/servers/${serverId}/channels/${channelId}/unmute`,
      { targetUserId },
    );
  }

  async deafen(
    serverId: string,
    channelId: string,
    targetUserId: string,
  ): Promise<void> {
    await this.client.post(
      `/v1/servers/${serverId}/channels/${channelId}/deafen`,
      { targetUserId },
    );
  }

  async undeafen(
    serverId: string,
    channelId: string,
    targetUserId: string,
  ): Promise<void> {
    await this.client.post(
      `/v1/servers/${serverId}/channels/${channelId}/undeafen`,
      { targetUserId },
    );
  }

  async listMessages(
    serverId: string,
    channelId: string,
    opts: { before?: string; after?: string; limit?: number } = {},
  ): Promise<ListChannelMessagesResponse> {
    const params: Record<string, string | number> = {};
    if (opts.before) params.before = opts.before;
    if (opts.after) params.after = opts.after;
    if (opts.limit !== undefined) params.limit = opts.limit;
    const response = await this.client.get(
      `/v1/servers/${serverId}/channels/${channelId}/messages`,
      { params },
    );
    return response.data.data;
  }

  async searchMessages(
    serverId: string,
    channelId: string,
    query: string,
    page: number = 1,
    perPage: number = 25,
  ): Promise<SearchChannelMessagesResponse> {
    const response = await this.client.get(
      `/v1/servers/${serverId}/channels/${channelId}/messages/search`,
      { params: { q: query, page, per_page: perPage } },
    );
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async sendMessage(
    serverId: string,
    channelId: string,
    content: string,
    attachmentIds?: string[],
  ): Promise<ChannelMessage> {
    const response = await this.client.post(
      `/v1/servers/${serverId}/channels/${channelId}/messages`,
      { content, attachmentIds },
    );
    return response.data.data;
  }

  async editMessage(
    serverId: string,
    channelId: string,
    messageId: string,
    content: string,
    addAttachmentIds?: string[],
    removeAttachmentIds?: string[],
  ): Promise<void> {
    await this.client.patch(
      `/v1/servers/${serverId}/channels/${channelId}/messages/${messageId}`,
      {
        content,
        addAttachmentIds: addAttachmentIds ?? [],
        removeAttachmentIds: removeAttachmentIds ?? [],
      },
    );
  }

  async deleteMessage(
    serverId: string,
    channelId: string,
    messageId: string,
  ): Promise<void> {
    await this.client.delete(
      `/v1/servers/${serverId}/channels/${channelId}/messages/${messageId}`,
    );
  }

  async joinText(
    serverId: string,
    channelId: string,
    password?: string,
  ): Promise<void> {
    const body: { password?: string } = {};
    if (password !== undefined) body.password = password;
    await this.client.post(
      `/v1/servers/${serverId}/channels/${channelId}/join`,
      body,
    );
  }
}

export const channelAPI = new ChannelAPI();
