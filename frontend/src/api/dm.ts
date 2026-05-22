import axios, { AxiosError, AxiosInstance, AxiosRequestConfig } from "axios";

import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";
import { ChannelMessage } from "../types/channel";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

interface DmRow {
  id: string;
  authorId: string;
  authorName?: string;
  recipientId: string;
  content: string;
  inviteServerId?: string;
  inviteServerName?: string;
  createdAt: string;
  editedAt: string | null;
}

const toChannelMessage = (r: DmRow): ChannelMessage => ({
  id: r.id,
  channelId: "",
  authorId: r.authorId,
  authorName: r.authorName,
  content: r.content,
  inviteServerId: r.inviteServerId,
  inviteServerName: r.inviteServerName,
  createdAt: r.createdAt,
  editedAt: r.editedAt,
});

class DmAPI {
  private client: AxiosInstance;

  constructor() {
    this.client = axios.create({
      baseURL: API_BASE_URL,
      headers: { "Content-Type": "application/json" },
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
          } catch (_e) {
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

  async listMessages(
    friendId: string,
    opts: { before?: string; after?: string; limit?: number } = {},
  ): Promise<{ messages: ChannelMessage[] }> {
    const params: Record<string, string | number> = {};
    if (opts.before) params.before = opts.before;
    if (opts.after) params.after = opts.after;
    if (opts.limit !== undefined) params.limit = opts.limit;
    const response = await this.client.get(
      `/v1/users/me/dms/${friendId}/messages`,
      { params },
    );
    return {
      messages: (response.data.data.messages as DmRow[]).map(toChannelMessage),
    };
  }

  async sendMessage(friendId: string, content: string): Promise<ChannelMessage> {
    const response = await this.client.post(
      `/v1/users/me/dms/${friendId}/messages`,
      { content },
    );
    return toChannelMessage(response.data.data);
  }

  async sendServerInvite(
    friendId: string,
    serverId: string,
  ): Promise<ChannelMessage> {
    const response = await this.client.post(
      `/v1/users/me/dms/${friendId}/server-invite`,
      { serverId },
    );
    return toChannelMessage(response.data.data);
  }

  async acceptServerInvite(messageId: string): Promise<{ serverId: string }> {
    const response = await this.client.post(
      `/v1/users/me/dms/messages/${messageId}/accept-invite`,
    );
    return { serverId: response.data.data.serverId };
  }

  async editMessage(messageId: string, content: string): Promise<void> {
    await this.client.patch(`/v1/users/me/dms/messages/${messageId}`, {
      content,
    });
  }

  async deleteMessage(messageId: string): Promise<void> {
    await this.client.delete(`/v1/users/me/dms/messages/${messageId}`);
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
