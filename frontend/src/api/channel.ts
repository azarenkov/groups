import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
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
} from "../types/channel";
import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

class ChannelAPI {
  private client: AxiosInstance;

  constructor() {
    this.client = axios.create({
      baseURL: API_BASE_URL,
      headers: {
        "Content-Type": "application/json",
      },
      timeout: 10000,
    });

    this.client.interceptors.response.use(
      (response) => response,
      async (error: AxiosError) => {
        const originalRequest = error.config as AxiosRequestConfig & {
          _retry?: boolean;
        };

        if (error.response?.status === 401 && !originalRequest._retry) {
          originalRequest._retry = true;
          try {
            const { authAPI } = await import("./auth");
            await authAPI.handleRefreshFlow();

            const tokens = useAuthStore.getState().tokens;
            if (tokens?.accessToken) {
              originalRequest.headers = originalRequest.headers || {};
              originalRequest.headers["Authorization"] =
                `Bearer ${tokens.accessToken}`;
            }
            return this.client.request(originalRequest);
          } catch (e) {
            throw toApiError(error);
          }
        }

        throw toApiError(error);
      },
    );
  }

  setAuthToken(token: string) {
    this.client.defaults.headers.common["Authorization"] = `Bearer ${token}`;
  }

  removeAuthToken() {
    delete this.client.defaults.headers.common["Authorization"];
  }

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
  ): Promise<ChannelMessage> {
    const response = await this.client.post(
      `/v1/servers/${serverId}/channels/${channelId}/messages`,
      { content },
    );
    return response.data.data;
  }

  async editMessage(
    serverId: string,
    channelId: string,
    messageId: string,
    content: string,
  ): Promise<void> {
    await this.client.patch(
      `/v1/servers/${serverId}/channels/${channelId}/messages/${messageId}`,
      { content },
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
