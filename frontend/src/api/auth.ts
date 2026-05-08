import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import { serverAPI } from "./server";
import { roomAPI } from "./room";
import { useAuthStore } from "../store/authStore";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

class AuthAPI {
  private client: AxiosInstance;
  private refreshInProgress: boolean = false;
  private refreshPromise: Promise<void> | null = null;

  constructor() {
    this.client = axios.create({
      baseURL: API_BASE_URL,
      headers: {
        "Content-Type": "application/json",
      },
      timeout: 10000,
    });

    // Response interceptor for handling errors and automatic refresh
    this.client.interceptors.response.use(
      (response) => response,
      async (error: AxiosError) => {
        const originalRequest = error.config as AxiosRequestConfig & {
          _retry?: boolean;
        };

        // If unauthorized and we haven't retried yet — attempt refresh flow
        if (error.response?.status === 401 && !originalRequest._retry) {
          originalRequest._retry = true;
          try {
            await this.handleRefreshFlow();
            const tokens = useAuthStore.getState().tokens;
            if (tokens?.accessToken) {
              originalRequest.headers = originalRequest.headers || {};
              originalRequest.headers["Authorization"] =
                `Bearer ${tokens.accessToken}`;
            }
            return this.client.request(originalRequest);
          } catch (e) {
            // Refresh failed — logout handled inside handleRefreshFlow
            throw error;
          }
        }

        // Fallback: extract possible server error message safely
        const respData = error.response ? (error.response.data as any) : null;
        if (respData?.error) {
          throw new Error(respData.error);
        }

        throw error;
      },
    );
  }

  // Public: register
  async register(name: string, email: string, password: string) {
    const response = await this.client.post("/v1/auth/register", {
      name,
      email,
      password,
    });
    return response.data.data;
  }

  // Public: login
  async login(email: string, password: string) {
    const response = await this.client.post("/v1/auth/login", {
      email,
      password,
    });
    return response.data.data;
  }

  // Public: call refresh endpoint (doesn't update store itself)
  async refresh(): Promise<{ accessToken: string; refreshToken: string }> {
    const tokens = useAuthStore.getState().tokens;
    if (!tokens?.refreshToken) {
      throw new Error("No refresh token available");
    }

    const response = await this.client.post("/v1/auth/refresh", {
      refreshToken: tokens.refreshToken,
    });
    return response.data.data;
  }

  // Public: perform refresh flow once (with concurrency control)
  public async handleRefreshFlow(): Promise<void> {
    if (this.refreshInProgress) {
      if (!this.refreshPromise) return;
      return this.refreshPromise;
    }

    this.refreshInProgress = true;
    this.refreshPromise = (async () => {
      try {
        const newTokens = await this.refresh();

        // update zustand store
        const user = useAuthStore.getState().user;
        useAuthStore.getState().setTokens(newTokens);
        if (user) {
          useAuthStore.getState().login(newTokens, user);
        } else {
          useAuthStore.getState().setTokens(newTokens);
        }

        // update axios defaults for all API clients
        this.setAuthToken(newTokens.accessToken);
      } catch (err) {
        // If refresh fails, logout and clear tokens from clients
        useAuthStore.getState().logout();
        this.removeAuthToken();
        serverAPI.removeAuthToken();
        roomAPI.removeAuthToken();
        throw err;
      } finally {
        this.refreshInProgress = false;
        this.refreshPromise = null;
      }
    })();

    return this.refreshPromise;
  }

  // Set auth token for this client and other API clients
  setAuthToken(token: string) {
    this.client.defaults.headers.common["Authorization"] = `Bearer ${token}`;
    serverAPI.setAuthToken(token);
    roomAPI.setAuthToken(token);
  }

  // Remove auth token from this client and other APIs
  removeAuthToken() {
    delete this.client.defaults.headers.common["Authorization"];
    serverAPI.removeAuthToken();
    roomAPI.removeAuthToken();
  }
}

export const authAPI = new AuthAPI();
