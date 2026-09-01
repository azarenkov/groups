import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

export interface CallParticipant {
  userId: string;
  userName: string;
}

export interface CallToken {
  token: string;
  url: string;
  participants: CallParticipant[];
}

class CallAPI {
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

  async start(
    recipientIds: string[],
  ): Promise<{ callId: string } & CallToken> {
    const response = await this.client.post("/v1/calls", { recipientIds });
    return response.data.data;
  }

  async accept(callId: string): Promise<CallToken> {
    const response = await this.client.post(`/v1/calls/${callId}/accept`, {});
    return response.data.data;
  }

  async getActive(): Promise<({ callId: string } & CallToken) | null> {
    const response = await this.client.get("/v1/calls/active");
    return response.data.data ?? null;
  }

  async activeCallWith(friendId: string): Promise<string | null> {
    const response = await this.client.get(`/v1/calls/with/${friendId}`);
    return response.data.data?.callId ?? null;
  }

  async join(callId: string): Promise<CallToken> {
    const response = await this.client.post(`/v1/calls/${callId}/join`, {});
    return response.data.data;
  }

  async logCall(
    callId: string,
    recipientId: string,
    status: string,
    durationSeconds?: number,
  ): Promise<void> {
    await this.client.post("/v1/calls/log", {
      callId,
      recipientId,
      status,
      durationSeconds: durationSeconds ?? null,
    });
  }

  async decline(callId: string): Promise<void> {
    await this.client.post(`/v1/calls/${callId}/decline`, {});
  }

  async leave(callId: string): Promise<void> {
    await this.client.post(`/v1/calls/${callId}/leave`, {});
  }

  async addParticipants(callId: string, recipientIds: string[]): Promise<void> {
    await this.client.post(`/v1/calls/${callId}/participants`, { recipientIds });
  }
}

export const callAPI = new CallAPI();
