import axios, { AxiosError, AxiosInstance, AxiosRequestConfig } from "axios";

import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";
import {
  FriendshipStatus,
  FriendIdsResponse,
  ListFriendshipsResponse,
} from "../types/friendship";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

class FriendshipAPI {
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

  async sendRequest(username: string): Promise<void> {
    await this.client.post("/v1/users/me/friends", { username });
  }

  async accept(friendId: string): Promise<void> {
    await this.client.post(`/v1/users/me/friends/${friendId}/accept`);
  }

  async remove(friendId: string): Promise<void> {
    await this.client.delete(`/v1/users/me/friends/${friendId}`);
  }

  async list(
    opts: { status?: FriendshipStatus; cursor?: string | null; limit?: number } = {},
  ): Promise<ListFriendshipsResponse> {
    const response = await this.client.get("/v1/users/me/friends", {
      params: {
        status: opts.status,
        cursor: opts.cursor ?? undefined,
        limit: opts.limit,
      },
    });
    return response.data.data;
  }

  async listIds(): Promise<FriendIdsResponse> {
    const response = await this.client.get("/v1/users/me/friends/ids");
    return response.data.data;
  }

  async searchUsers(
    query: string,
    limit: number = 8,
  ): Promise<{ id: string; name: string }[]> {
    const response = await this.client.get("/v1/users/search", {
      params: { q: query, limit },
    });
    return response.data.data.users;
  }
}

export const friendshipAPI = new FriendshipAPI();
