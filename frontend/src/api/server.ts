import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import {
  ListServersResponse,
  CreateServerRequest,
  CreateServerResponse,
} from "../types/server";
import { useAuthStore } from "../store/authStore";

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

    // Response interceptor for handling 401 -> refresh -> retry
    this.client.interceptors.response.use(
      (response) => response,
      async (error: AxiosError) => {
        const originalRequest = error.config as AxiosRequestConfig & {
          _retry?: boolean;
        };

        // Debug log
        console.debug(
          "[serverAPI] response interceptor, status:",
          error.response?.status,
        );

        if (error.response?.status === 401 && !originalRequest._retry) {
          originalRequest._retry = true;
          try {
            console.debug(
              "[serverAPI] 401 received, attempting token refresh...",
            );
            // dynamic import to avoid circular dependency
            const { authAPI } = await import("./auth");
            await authAPI.handleRefreshFlow();

            const tokens = useAuthStore.getState().tokens;
            if (tokens?.accessToken) {
              originalRequest.headers = originalRequest.headers || {};
              originalRequest.headers["Authorization"] =
                `Bearer ${tokens.accessToken}`;
            }

            console.debug(
              "[serverAPI] retrying original request after refresh",
            );
            return this.client.request(originalRequest);
          } catch (e) {
            console.debug(
              "[serverAPI] refresh failed or retry failed, propagating error",
            );
            // authAPI should have performed logout on refresh failure
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

  async listServers(
    page: number = 1,
    perPage: number = 20,
  ): Promise<ListServersResponse> {
    const response = await this.client.get("/v1/server/list", {
      params: { page, per_page: perPage },
    });
    return response.data.data;
  }

  async createServer(data: CreateServerRequest): Promise<CreateServerResponse> {
    const response = await this.client.post("/v1/server/create", data);
    return response.data.data;
  }

  async deleteServer(serverId: string): Promise<void> {
    await this.client.delete(`/v1/server/${serverId}`);
  }
}

export const serverAPI = new ServerAPI();
