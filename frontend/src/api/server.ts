import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import {
  ListServersResponse,
  ListServerMembersResponse,
  CreateServerRequest,
  CreateServerResponse,
  InviteLink,
  InvitePreview,
  JoinInviteResponse,
} from "../types/server";
import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";
import { rateLimitRetry } from "../utils/rateLimitRetry";

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

        const retried = rateLimitRetry(this.client, error);
        if (retried) return retried;
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

  async uploadLogo(serverId: string, file: File): Promise<void> {
    const form = new FormData();
    form.append("file", file, file.name);
    await this.client.post(`/v1/servers/${serverId}/logo`, form, {
      headers: { "Content-Type": "multipart/form-data" },
      timeout: 60000,
    });
  }

  async deleteLogo(serverId: string): Promise<void> {
    await this.client.delete(`/v1/servers/${serverId}/logo`);
  }

  async downloadLogo(serverId: string): Promise<Blob> {
    const response = await this.client.get(`/v1/servers/${serverId}/logo`, {
      responseType: "blob",
      timeout: 30000,
    });
    return response.data;
  }


  async listRoles(serverId: string): Promise<{
    roles: { id: string; name: string; color: string; permissions: number; position: number }[];
    assignments: { userId: string; roleId: string }[];
  }> {
    const response = await this.client.get(`/v1/servers/${serverId}/roles`);
    return response.data.data;
  }

  async createRole(
    serverId: string,
    name: string,
    color: string,
    permissions: number,
  ): Promise<{ id: string }> {
    const response = await this.client.post(`/v1/servers/${serverId}/roles`, {
      name,
      color,
      permissions,
    });
    return response.data.data;
  }

  async updateRole(
    serverId: string,
    roleId: string,
    patch: {
      name?: string;
      color?: string;
      permissionsAdd?: number;
      permissionsRemove?: number;
    },
  ): Promise<void> {
    await this.client.put(`/v1/servers/${serverId}/roles/${roleId}`, patch);
  }

  async deleteRole(serverId: string, roleId: string): Promise<void> {
    await this.client.delete(`/v1/servers/${serverId}/roles/${roleId}`);
  }

  async kickMember(serverId: string, memberId: string): Promise<void> {
    await this.client.delete(`/v1/servers/${serverId}/members/${memberId}`);
  }

  async swapRoles(serverId: string, roleA: string, roleB: string): Promise<void> {
    await this.client.post(`/v1/servers/${serverId}/roles/${roleA}/swap/${roleB}`);
  }

  async assignRole(serverId: string, memberId: string, roleId: string): Promise<void> {
    await this.client.put(
      `/v1/servers/${serverId}/members/${memberId}/roles/${roleId}`,
    );
  }

  async revokeRole(serverId: string, memberId: string, roleId: string): Promise<void> {
    await this.client.delete(
      `/v1/servers/${serverId}/members/${memberId}/roles/${roleId}`,
    );
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

  async createInviteLink(
    serverId: string,
    opts: { name?: string; expiresDays?: number; maxUses?: number } = {},
  ): Promise<InviteLink> {
    const response = await this.client.post(
      `/v1/servers/${serverId}/invite-links`,
      {
        name: opts.name,
        expiresDays: opts.expiresDays,
        maxUses: opts.maxUses,
      },
    );
    return response.data.data;
  }

  async listInviteLinks(
    serverId: string,
    opts: { cursor?: string | null; limit?: number } = {},
  ): Promise<{ primary: InviteLink; links: InviteLink[]; nextCursor: string | null }> {
    const response = await this.client.get(
      `/v1/servers/${serverId}/invite-links`,
      { params: { cursor: opts.cursor ?? undefined, limit: opts.limit } },
    );
    return response.data.data;
  }

  async revokeInviteLink(serverId: string, token: string): Promise<void> {
    await this.client.delete(
      `/v1/servers/${serverId}/invite-links/${encodeURIComponent(token)}`,
    );
  }

  async previewInvite(token: string): Promise<InvitePreview> {
    const response = await this.client.get(
      `/v1/invites/${encodeURIComponent(token)}`,
    );
    return response.data.data;
  }

  async joinByInvite(token: string): Promise<JoinInviteResponse> {
    const response = await this.client.post(
      `/v1/invites/${encodeURIComponent(token)}/join`,
    );
    return response.data.data;
  }

  async listVoicePresence(serverId: string): Promise<{
    items: {
      channelId: string;
      userId: string;
      userName: string;
      isMuted: boolean;
      mutedBy?: string | null;
      deafenedBy?: string | null;
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
