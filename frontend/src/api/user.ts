import axios, { AxiosError, AxiosInstance, AxiosRequestConfig } from "axios";

import { useAuthStore } from "../store/authStore";
import { ApiError, toApiError } from "../utils/apiError";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

export interface Reauth {
  password?: string;
  googleIdToken?: string;
}

export interface AppearanceSettings {
  theme: string;
  accent: string;
  chatFontSize: number;
  reduceMotion: boolean;
}

export interface VoiceSettings {
  micVolume: number;
  outputVolume: number;
  noiseSuppression: boolean;
  autoSensitivity: boolean;
  sensitivity: number;
  pushToTalk: boolean;
}

export interface Connection {
  provider: string;
  displayName: string;
  url?: string | null;
}

export interface MeProfile {
  id: string;
  name: string;
  email: string;
  locale: string;
  emailVerified: boolean;
  passwordChangedAt: string | null;
  linkedProviders: string[];
  bio: string;
  showActivity: boolean;
  timezone: string;
  profileUpdatedAt: string;
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

  async listConnections(): Promise<Connection[]> {
    const response = await this.client.get("/v1/users/me/connections");
    return response.data.data.connections;
  }

  async startConnection(provider: string): Promise<string> {
    const response = await this.client.post(
      `/v1/users/me/connections/${provider}/start`,
      {},
    );
    return response.data.data.url;
  }

  async removeConnection(provider: string): Promise<void> {
    await this.client.delete(`/v1/users/me/connections/${provider}`);
  }

  async userConnections(userId: string): Promise<Connection[]> {
    const response = await this.client.get(`/v1/users/${userId}/connections`);
    return response.data.data.connections;
  }

  async getAppearance(): Promise<AppearanceSettings | null> {
    try {
      const response = await this.client.get("/v1/users/me/appearance");
      return response.data.data;
    } catch (e) {
      if (e instanceof ApiError && e.code === "APPEARANCE_NOT_SET") return null;
      throw e;
    }
  }

  async updateAppearance(
    patch: Partial<AppearanceSettings>,
  ): Promise<AppearanceSettings> {
    const response = await this.client.patch("/v1/users/me/appearance", patch, {
      _retry: true,
    } as AxiosRequestConfig);
    return response.data.data;
  }

  async getVoiceSettings(): Promise<VoiceSettings | null> {
    try {
      const response = await this.client.get("/v1/users/me/voice");
      return response.data.data;
    } catch (e) {
      if (e instanceof ApiError && e.code === "VOICE_SETTINGS_NOT_SET") {
        return null;
      }
      throw e;
    }
  }

  async updateVoiceSettings(
    patch: Partial<VoiceSettings>,
  ): Promise<VoiceSettings> {
    const response = await this.client.patch("/v1/users/me/voice", patch, {
      _retry: true,
    } as AxiosRequestConfig);
    return response.data.data;
  }

  async uploadAvatar(file: File): Promise<void> {
    const form = new FormData();
    form.append("file", file, file.name);
    await this.client.post("/v1/users/me/avatar", form, {
      headers: { "Content-Type": "multipart/form-data" },
      timeout: 60000,
    });
  }

  async deleteAvatar(): Promise<void> {
    await this.client.delete("/v1/users/me/avatar");
  }

  async changePassword(
    reauth: Reauth,
    newPassword: string,
  ): Promise<{ accessToken: string; refreshToken: string; tokenType: string; expiresIn: number }> {
    const response = await this.client.patch("/v1/users/me/password", {
      currentPassword: reauth.password,
      googleIdToken: reauth.googleIdToken,
      newPassword,
    });
    return response.data.data;
  }

  async requestEmailChange(reauth: Reauth, newEmail: string): Promise<void> {
    await this.client.post("/v1/users/me/email/change", { ...reauth, newEmail });
  }

  async confirmEmailChange(verificationCode: string): Promise<string> {
    const response = await this.client.post("/v1/users/me/email/confirm", { verificationCode });
    return response.data.data.email as string;
  }

  async disableAccount(reauth: Reauth): Promise<void> {
    await this.client.post("/v1/users/me/disable", reauth);
  }

  async deleteAccount(reauth: Reauth): Promise<void> {
    await this.client.delete("/v1/users/me", { data: reauth });
  }

  async updateProfile(patch: { bio?: string; showActivity?: boolean; timezone?: string }): Promise<{ bio: string; showActivity: boolean; profileUpdatedAt: string }> {
    const response = await this.client.patch("/v1/users/me/profile", patch);
    return response.data.data;
  }

  async rename(name: string): Promise<string> {
    const response = await this.client.patch("/v1/users/me/name", { name });
    return response.data.data.name;
  }

  async downloadAvatar(userId: string): Promise<Blob> {
    const response = await this.client.get(`/v1/users/${userId}/avatar`, {
      responseType: "blob",
      timeout: 30000,
    });
    return response.data;
  }

  async getUnread(): Promise<{
    channels: { channelId: string; serverId: string; count: number }[];
    dms: { friendId: string; count: number }[];
  }> {
    const response = await this.client.get("/v1/me/unread");
    return response.data.data;
  }

  async markChannelRead(channelId: string): Promise<void> {
    await this.client.post(`/v1/channels/${channelId}/read`);
  }

  async markDmRead(friendId: string): Promise<void> {
    await this.client.post(`/v1/dms/${friendId}/read`);
  }

  async getDmReadState(friendId: string): Promise<{ readAt: string | null }> {
    const response = await this.client.get(`/v1/dms/${friendId}/read-state`);
    return response.data.data;
  }

  async getChannelReadStates(
    channelId: string,
  ): Promise<{ members: { userId: string; readAt: string | null }[] }> {
    const response = await this.client.get(
      `/v1/channels/${channelId}/read-states`,
    );
    return response.data.data;
  }

  async setAppearOffline(appearOffline: boolean): Promise<void> {
    await this.client.post("/v1/me/presence", { appearOffline });
  }

  async getNotificationPreferences(): Promise<{
    preferences: {
      scopeType: string;
      scopeId: string;
      level: string;
      version?: number;
    }[];
  }> {
    const response = await this.client.get("/v1/notifications/preferences");
    return response.data.data;
  }

  async setNotificationPreference(
    scopeType: "channel" | "dm",
    scopeId: string,
    level: "all" | "mentions" | "none",
  ): Promise<number | undefined> {
    const response = await this.client.put("/v1/notifications/preferences", {
      scopeType,
      scopeId,
      level,
    });
    return response.data?.data?.version;
  }
}

export const userAPI = new UserAPI();
