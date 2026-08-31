import axios, { AxiosError, AxiosInstance, AxiosRequestConfig } from "axios";

import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

export interface ActiveFriend {
  userId: string;
  userName: string;
  channelId: string;
  channelName: string;
  serverId: string;
  serverName: string;
  joinedAt: string;
  isMuted: boolean;
  cameraOn: boolean;
  screenOn: boolean;
  profileUpdatedAt: string | null;
}

export interface PrivacySettings {
  dmPolicy: "all" | "friends" | "no_one";
  acceptFriendRequests: boolean;
}

export interface PublicProfile {
  id: string;
  name: string;
  bio: string;
  profileUpdatedAt: string | null;
  active: boolean;
  timezone: string;
  memberSince: string | null;
  mutualServers: { id: string; name: string; accentColor: string | null }[];
  voiceRoom: { channelId: string; channelName: string; serverId: string; serverName: string } | null;
}

export interface RecentRoom {
  channelId: string;
  channelName: string;
  serverId: string;
  serverName: string;
  liveCount: number;
  lastJoin: string;
}

export interface HomeOverview {
  activeFriends: ActiveFriend[];
  recentRooms: RecentRoom[];
}

class HomeAPI {
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

  async privacy(): Promise<PrivacySettings> {
    const response = await this.client.get("/v1/users/me/privacy");
    return response.data.data;
  }

  async updatePrivacy(patch: Partial<PrivacySettings>): Promise<PrivacySettings> {
    const response = await this.client.patch("/v1/users/me/privacy", patch);
    return response.data.data;
  }

  async profile(userId: string): Promise<PublicProfile> {
    const response = await this.client.get(`/v1/users/${userId}/profile`);
    return response.data.data;
  }

  async overview(): Promise<HomeOverview> {
    const response = await this.client.get("/v1/users/me/home");
    return response.data.data;
  }
}

export const homeAPI = new HomeAPI();
