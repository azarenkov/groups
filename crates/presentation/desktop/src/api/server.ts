import axios, { AxiosInstance } from "axios";
import {
  ListServersResponse,
  CreateServerRequest,
  CreateServerResponse,
} from "../types/server";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

class ServerAPI {
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

  async listServers(
    page: number = 1,
    perPage: number = 20,
  ): Promise<ListServersResponse> {
    const response = await this.client.get("/v1/server/list", {
      params: { page, per_page: perPage },
    });
    return response.data.data;
  }

  async createServer(data: CreateServerRequest): Promise<CreateServerResponse> {
    const response = await this.client.post("/v1/server/create", data);
    return response.data.data;
  }

  async deleteServer(serverId: string): Promise<void> {
    await this.client.delete(`/v1/server/${serverId}`);
  }
}

export const serverAPI = new ServerAPI();
