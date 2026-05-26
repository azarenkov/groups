import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import { serverAPI } from "./server";
import { channelAPI } from "./channel";
import { friendshipAPI } from "./friendship";
import { dmAPI } from "./dm";
import { userAPI } from "./user";
import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";

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
        const url = originalRequest.url ?? "";
        const skipRefresh =
          url.endsWith("/v1/auth/refresh") ||
          url.endsWith("/v1/auth/login") ||
          url.endsWith("/v1/auth/register");

        if (
          error.response?.status === 401 &&
          !originalRequest._retry &&
          !skipRefresh
        ) {
          originalRequest._retry = true;
          try {
            await this.handleRefreshFlow();
            const tokens = useAuthStore.getState().tokens;
            if (tokens?.accessToken) {
              originalRequest.headers = originalRequest.headers || {};
              originalRequest.headers["Authorization"] =
                `Bearer ${tokens.accessToken}`;
            }
            if (
              url.endsWith("/v1/auth/logout") &&
              tokens?.refreshToken
            ) {
              originalRequest.data = JSON.stringify({
                refreshToken: tokens.refreshToken,
              });
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

  async register(
    name: string,
    email: string,
    password: string,
    locale?: string,
  ): Promise<TokenPair> {
    const response = await this.client.post("/v1/auth/register", {
      name,
      email,
      password,
      ...(locale ? { locale } : {}),
    });
    return response.data.data;
  }

  async updateLocale(locale: string): Promise<void> {
    const tokens = useAuthStore.getState().tokens;
    await this.client.patch(
      "/v1/me/locale",
      { locale },
      {
        headers: tokens?.accessToken
          ? { Authorization: `Bearer ${tokens.accessToken}` }
          : undefined,
      },
    );
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
    return this.refreshWith(tokens.refreshToken);
  }

  async refreshWith(refreshToken: string): Promise<TokenPair> {
    const response = await this.client.post("/v1/auth/refresh", {
      refreshToken,
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

  public async handleRefreshFlow(
    opts: { logoutOnFailure?: boolean } = {},
  ): Promise<void> {
    const logoutOnFailure = opts.logoutOnFailure ?? true;

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
        if (logoutOnFailure) {
          useAuthStore.getState().logout();
          this.removeAuthToken();
        }
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
    friendshipAPI.setAuthToken(token);
    dmAPI.setAuthToken(token);
    userAPI.setAuthToken(token);
  }

  removeAuthToken() {
    delete this.client.defaults.headers.common["Authorization"];
    serverAPI.removeAuthToken();
    channelAPI.removeAuthToken();
    friendshipAPI.removeAuthToken();
    dmAPI.removeAuthToken();
    userAPI.removeAuthToken();
  }
}

export const authAPI = new AuthAPI();
