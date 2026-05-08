import axios, { AxiosInstance, AxiosRequestConfig, AxiosError } from "axios";
import {
  ListRoomsResponse,
  CreateRoomRequest,
  CreateRoomResponse,
  JoinRoomRequest,
  JoinRoomResponse,
} from "../types/room";
import { useAuthStore } from "../store/authStore";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

class RoomAPI {
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
          "[roomAPI] response interceptor, status:",
          error.response?.status,
        );

        if (error.response?.status === 401 && !originalRequest._retry) {
          originalRequest._retry = true;
          try {
            console.debug(
              "[roomAPI] 401 received, attempting token refresh...",
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

            console.debug("[roomAPI] retrying original request after refresh");
            return this.client.request(originalRequest);
          } catch (e) {
            console.debug(
              "[roomAPI] refresh failed or retry failed, propagating error",
            );
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

  async listRoomsByServer(
    serverId: string,
    page: number = 1,
    perPage: number = 50,
  ): Promise<ListRoomsResponse> {
    const response = await this.client.get(`/v1/server/${serverId}/rooms`, {
      params: { page, per_page: perPage },
    });
    return response.data.data;
  }

  async createRoom(data: CreateRoomRequest): Promise<CreateRoomResponse> {
    const response = await this.client.post("/v1/room/create", {
      serverId: data.serverId,
      name: data.name,
      roomType: data.roomType,
      password: data.password,
    });
    return response.data.data;
  }

  async deleteRoom(roomId: string): Promise<void> {
    await this.client.delete(`/v1/room/${roomId}`);
  }

  async joinRoom(data: JoinRoomRequest): Promise<JoinRoomResponse> {
    const response = await this.client.post(`/v1/room/${data.roomId}/join`, {
      serverId: data.serverId,
    });
    return response.data.data;
  }
}

export const roomAPI = new RoomAPI();
