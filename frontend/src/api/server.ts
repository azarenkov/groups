import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import {
  ListServersResponse,
  ListServerMembersResponse,
  CreateServerRequest,
  CreateServerResponse,
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

  async deleteServer(serverId: string): Promise<void> {
    await this.client.delete(`/v1/servers/${serverId}`);
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
}

export const serverAPI = new ServerAPI();
