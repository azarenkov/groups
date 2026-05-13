import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import { serverAPI } from "./server";
import { channelAPI } from "./channel";
import { useAuthStore } from "../store/authStore";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

export interface TokenPair {
  accessToken: string;
  refreshToken: string;
  tokenType: string;
  expiresIn: number;
  requiresVerification?: boolean;
}

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

    this.client.interceptors.response.use(
      (response) => response,
      async (error: AxiosError) => {
        const originalRequest = error.config as AxiosRequestConfig & {
          _retry?: boolean;
        };

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

  async register(
    name: string,
    email: string,
    password: string,
  ): Promise<TokenPair> {
    const response = await this.client.post("/v1/auth/register", {
      name,
      email,
      password,
    });
    return response.data.data;
  }

  async requestVerificationCode(
    channel: "email" | "phone" = "email",
  ): Promise<void> {
    const tokens = useAuthStore.getState().tokens;
    await this.client.post(
      "/v1/auth/request_verification_code",
      { channel },
      {
        headers: tokens?.accessToken
          ? { Authorization: `Bearer ${tokens.accessToken}` }
          : undefined,
      },
    );
  }

  async confirmRegistration(verificationCode: string): Promise<void> {
    const tokens = useAuthStore.getState().tokens;
    await this.client.post(
      "/v1/auth/confirm_registration",
      { verificationCode },
      {
        headers: tokens?.accessToken
          ? { Authorization: `Bearer ${tokens.accessToken}` }
          : undefined,
      },
    );
  }

  async login(email: string, password: string): Promise<TokenPair> {
    const response = await this.client.post("/v1/auth/login", {
      email,
      password,
    });
    return response.data.data;
  }

  async refresh(): Promise<TokenPair> {
    const tokens = useAuthStore.getState().tokens;
    if (!tokens?.refreshToken) {
      throw new Error("No refresh token available");
    }

    const response = await this.client.post("/v1/auth/refresh", {
      refreshToken: tokens.refreshToken,
    });
    return response.data.data;
  }

  async logout(): Promise<void> {
    const tokens = useAuthStore.getState().tokens;
    if (!tokens?.refreshToken || !tokens?.accessToken) {
      return;
    }

    try {
      await this.client.post(
        "/v1/auth/logout",
        { refreshToken: tokens.refreshToken },
        {
          headers: { Authorization: `Bearer ${tokens.accessToken}` },
        },
      );
    } finally {
      this.removeAuthToken();
    }
  }

  public async handleRefreshFlow(): Promise<void> {
    if (this.refreshInProgress) {
      if (!this.refreshPromise) return;
      return this.refreshPromise;
    }

    this.refreshInProgress = true;
    this.refreshPromise = (async () => {
      try {
        const newTokens = await this.refresh();

        useAuthStore.getState().setTokens(newTokens);
        this.setAuthToken(newTokens.accessToken);
      } catch (err) {
        useAuthStore.getState().logout();
        this.removeAuthToken();
        serverAPI.removeAuthToken();
        channelAPI.removeAuthToken();
        throw err;
      } finally {
        this.refreshInProgress = false;
        this.refreshPromise = null;
      }
    })();

    return this.refreshPromise;
  }

  setAuthToken(token: string) {
    this.client.defaults.headers.common["Authorization"] = `Bearer ${token}`;
    serverAPI.setAuthToken(token);
    channelAPI.setAuthToken(token);
  }

  removeAuthToken() {
    delete this.client.defaults.headers.common["Authorization"];
    serverAPI.removeAuthToken();
    channelAPI.removeAuthToken();
  }
}

export const authAPI = new AuthAPI();
