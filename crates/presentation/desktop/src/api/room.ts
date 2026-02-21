import axios, { AxiosInstance } from "axios";
import {
  ListRoomsResponse,
  CreateRoomRequest,
  CreateRoomResponse,
} from "../types/room";

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

    // Response interceptor for handling errors
    this.client.interceptors.response.use(
      (response) => response,
      (error) => {
        if (error.response?.data?.error) {
          throw new Error(error.response.data.error);
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
}

export const roomAPI = new RoomAPI();
