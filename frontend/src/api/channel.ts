import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import {
  ListChannelsResponse,
  CreateChannelRequest,
  CreateChannelResponse,
  JoinVoiceChannelRequest,
  JoinVoiceChannelResponse,
} from "../types/channel";
import { useAuthStore } from "../store/authStore";

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
            throw error;
          }
        }

        const respData = error.response ? (error.response.data as any) : null;
        if (respData?.error) {
          throw new Error(respData.error);
        }

        throw error;
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
    return response.data.data;
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
}

export const channelAPI = new ChannelAPI();
