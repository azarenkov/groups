import axios, { AxiosInstance } from "axios";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

class AuthAPI {
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

  async register(name: string, email: string, password: string) {
    const response = await this.client.post("/v1/auth/register", {
      name,
      email,
      password,
    });
    return response.data.data;
  }

  async login(email: string, password: string) {
    const response = await this.client.post("/v1/auth/login", {
      email,
      password,
    });
    return response.data.data;
  }

  setAuthToken(token: string) {
    this.client.defaults.headers.common["Authorization"] = `Bearer ${token}`;
  }

  removeAuthToken() {
    delete this.client.defaults.headers.common["Authorization"];
  }
}

export const authAPI = new AuthAPI();
