import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import {
  ListServersResponse,
  ListServerMembersResponse,
  CreateServerRequest,
  CreateServerResponse,
  InviteCode,
} from "../types/server";
import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

class ServerAPI {
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

  async listServers(
    page: number = 1,
    perPage: number = 20,
  ): Promise<ListServersResponse> {
    const response = await this.client.get("/v1/servers", {
      params: { page, per_page: perPage },
    });
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async createServer(data: CreateServerRequest): Promise<CreateServerResponse> {
    const response = await this.client.post("/v1/servers", data);
    return response.data.data;
  }

  async updateServer(
    serverId: string,
    data: { name: string; description?: string },
  ): Promise<void> {
    await this.client.patch(`/v1/servers/${serverId}`, data);
  }

  async createInviteCode(
    serverId: string,
    opts: { expiresDays?: number | null; maxUses?: number | null } = {},
  ): Promise<InviteCode> {
    const response = await this.client.post(
      `/v1/servers/${serverId}/invite-codes`,
      {
        expiresDays: opts.expiresDays ?? undefined,
        maxUses: opts.maxUses ?? undefined,
      },
    );
    return response.data.data;
  }

  async listInviteCodes(serverId: string): Promise<{ invites: InviteCode[] }> {
    const response = await this.client.get(
      `/v1/servers/${serverId}/invite-codes`,
    );
    return response.data.data;
  }

  async revokeInviteCode(serverId: string, code: string): Promise<void> {
    await this.client.delete(
      `/v1/servers/${serverId}/invite-codes/${encodeURIComponent(code)}`,
    );
  }

  async redeemInviteCode(code: string): Promise<{ serverId: string }> {
    const response = await this.client.post(
      `/v1/servers/invite-codes/${encodeURIComponent(code)}/redeem`,
    );
    return response.data.data;
  }

  async deleteServer(serverId: string): Promise<void> {
    await this.client.delete(`/v1/servers/${serverId}`);
  }

  async leave(serverId: string): Promise<void> {
    await this.client.delete(`/v1/servers/${serverId}/members/me`);
  }

  async listMembers(
    serverId: string,
    page: number = 1,
    perPage: number = 50,
  ): Promise<ListServerMembersResponse> {
    const response = await this.client.get(
      `/v1/servers/${serverId}/members`,
      { params: { page, per_page: perPage } },
    );
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async listAllMembers(serverId: string): Promise<{ id: string; name: string }[]> {
    const perPage = 50;
    const all: { id: string; name: string }[] = [];
    let page = 1;
    while (true) {
      const res = await this.listMembers(serverId, page, perPage);
      all.push(...res.members);
      const total = res.pagination?.totalPages ?? 1;
      if (page >= total) break;
      page++;
    }
    return all;
  }

  async listVoicePresence(serverId: string): Promise<{
    items: {
      channelId: string;
      userId: string;
      userName: string;
      isMuted: boolean;
      cameraOn: boolean;
      screenOn: boolean;
    }[];
    streamViewers: {
      channelId: string;
      streamerId: string;
      count: number;
    }[];
  }> {
    const response = await this.client.get(
      `/v1/servers/${serverId}/voice-presence`,
    );
    return {
      items: response.data.data.items,
      streamViewers: response.data.data.streamViewers ?? [],
    };
  }

  async updateStreamViewer(
    serverId: string,
    channelId: string,
    streamerId: string,
    viewerSessionId: string,
    watching: boolean,
  ): Promise<void> {
    await this.client.put(`/v1/servers/${serverId}/stream-viewers`, {
      channelId,
      streamerId,
      viewerSessionId,
      watching,
    });
  }

  async listSentInvites(serverId: string): Promise<{ recipientIds: string[] }> {
    const response = await this.client.get(
      `/v1/servers/${serverId}/sent-invites`,
    );
    return { recipientIds: response.data.data.recipientIds };
  }
}

export const serverAPI = new ServerAPI();
