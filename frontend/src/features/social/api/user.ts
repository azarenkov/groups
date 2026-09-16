import type { AxiosRequestConfig } from "axios";
import { http } from "../../../shared/api/http";

import { ApiError } from "../../../shared/lib/apiError";

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

export interface HotkeySettings {
  bindings: Record<string, string>;
}

export interface UserActivity {
  userId: string;
  gameId: string;
  name: string;
  startedAt: string;
  changedAt: string;
}

export interface ActivitySnapshot {
  asOf: string;
  activities: UserActivity[];
}

export interface VoiceSettings {
  micVolume: number;
  outputVolume: number;
  noiseMode: string;
  autoSensitivity: boolean;
  sensitivity: number;
  pushToTalk: boolean;
  pttKey: string;
}

export interface NotificationSettings {
  desktopEnabled: boolean;
  soundsEnabled: boolean;
  mentions: string;
  quietHoursEnabled: boolean;
  quietStart: string;
  quietEnd: string;
  callsEnabled: boolean;
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
  showGameActivity: boolean;
  timezone: string;
  profileUpdatedAt: string;
}

class UserAPI {
  private client = http;

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

  async setActivity(input: {
    gameId: string;
    name: string;
    startedAt: string;
    sessionId: string;
  }): Promise<UserActivity> {
    const response = await this.client.put("/v1/users/me/activity", input);
    return response.data.data;
  }

  async clearActivity(
    sessionId: string,
    accessToken?: string,
  ): Promise<{ occurredAt: string }> {
    const response = await this.client.delete("/v1/users/me/activity", {
      params: { session_id: sessionId },
      ...(accessToken
        ? { headers: { Authorization: `Bearer ${accessToken}` } }
        : {}),
    });
    return response.data.data;
  }

  async getActivities(ids: string[]): Promise<ActivitySnapshot> {
    if (ids.length === 0) return { asOf: new Date().toISOString(), activities: [] };
    const response = await this.client.get("/v1/users/activity", {
      params: { ids: ids.join(",") },
    });
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
    accessToken?: string,
  ): Promise<VoiceSettings> {
    const response = await this.client.patch("/v1/users/me/voice", patch, {
      _retry: true,
      ...(accessToken
        ? { headers: { Authorization: `Bearer ${accessToken}` } }
        : {}),
    } as AxiosRequestConfig);
    return response.data.data;
  }

  async getNotificationSettings(): Promise<NotificationSettings | null> {
    try {
      const response = await this.client.get("/v1/users/me/notifications");
      return response.data.data;
    } catch (e) {
      if (e instanceof ApiError && e.code === "NOTIFICATION_SETTINGS_NOT_SET") {
        return null;
      }
      throw e;
    }
  }

  async updateNotificationSettings(
    patch: Partial<NotificationSettings>,
  ): Promise<NotificationSettings> {
    const response = await this.client.patch(
      "/v1/users/me/notifications",
      patch,
      {
        _retry: true,
      } as AxiosRequestConfig,
    );
    return response.data.data;
  }

  async getHotkeySettings(): Promise<HotkeySettings | null> {
    try {
      const response = await this.client.get("/v1/users/me/hotkeys");
      return response.data.data;
    } catch (e) {
      if (e instanceof ApiError && e.code === "HOTKEY_SETTINGS_NOT_SET") {
        return null;
      }
      throw e;
    }
  }

  async updateHotkeySettings(
    bindings: Record<string, string | null>,
  ): Promise<HotkeySettings> {
    const response = await this.client.patch(
      "/v1/users/me/hotkeys",
      { bindings },
      {
        _retry: true,
      } as AxiosRequestConfig,
    );
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

  async updateProfile(patch: { bio?: string; showActivity?: boolean; showGameActivity?: boolean; timezone?: string }): Promise<{ bio: string; showActivity: boolean; showGameActivity: boolean; profileUpdatedAt: string }> {
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

  async getUnread(defaultLevel?: string): Promise<{
    channels: { channelId: string; serverId: string; count: number }[];
    dms: { friendId: string; count: number }[];
  }> {
    const response = await this.client.get("/v1/me/unread", {
      params: defaultLevel ? { default_level: defaultLevel } : undefined,
    });
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

  async presenceSnapshot(ids: string[]): Promise<{ online: string[] }> {
    if (ids.length === 0) return { online: [] };
    const response = await this.client.get("/v1/presence", {
      params: { ids: ids.join(",") },
    });
    return response.data;
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
