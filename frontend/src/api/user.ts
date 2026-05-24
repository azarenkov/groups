import axios, { AxiosError, AxiosInstance, AxiosRequestConfig } from "axios";

import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

export interface MeProfile {
  id: string;
  name: string;
  email: string;
}

class UserAPI {
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

  async getMe(): Promise<MeProfile> {
    const response = await this.client.get("/v1/users/me");
    return response.data.data;
  }
}

export const userAPI = new UserAPI();
