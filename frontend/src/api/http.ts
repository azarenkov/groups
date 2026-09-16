import axios, {
  AxiosError,
  AxiosRequestConfig,
  AxiosResponse,
  InternalAxiosRequestConfig,
} from "axios";

import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";
import { rateLimitRetry } from "../utils/rateLimitRetry";

export const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

const REQUEST_TIMEOUT_MS = 10000;
export const UPLOAD_TIMEOUT_MS = 120000;

const REFRESH_EXEMPT_PATHS = [
  "/v1/auth/refresh",
  "/v1/auth/login",
  "/v1/auth/register",
  "/v1/auth/google",
  "/v1/auth/google/complete",
];

type RetriableConfig = AxiosRequestConfig & { _retry?: boolean };

export const http = axios.create({
  baseURL: API_BASE_URL,
  headers: { "Content-Type": "application/json" },
  timeout: REQUEST_TIMEOUT_MS,
});

http.interceptors.request.use((config: InternalAxiosRequestConfig) => {
  if (!config.headers.Authorization) {
    const accessToken = useAuthStore.getState().tokens?.accessToken;
    if (accessToken) config.headers.Authorization = `Bearer ${accessToken}`;
  }
  return config;
});

http.interceptors.response.use(
  (response) => response,
  async (error: AxiosError) => {
    const request = error.config as RetriableConfig | undefined;
    if (request && error.response?.status === 401 && !request._retry && refreshAllowed(request)) {
      request._retry = true;
      try {
        return await retryWithFreshToken(request);
      } catch {
        throw toApiError(error);
      }
    }
    const retried = rateLimitRetry(http, error);
    if (retried) return retried;
    throw toApiError(error);
  },
);

function refreshAllowed(request: RetriableConfig): boolean {
  const url = request.url ?? "";
  return !REFRESH_EXEMPT_PATHS.some((path) => url.endsWith(path));
}

async function retryWithFreshToken(request: RetriableConfig): Promise<AxiosResponse> {
  const { authAPI } = await import("./auth");
  await authAPI.handleRefreshFlow();
  const tokens = useAuthStore.getState().tokens;
  if (tokens?.accessToken) {
    request.headers = { ...request.headers, Authorization: `Bearer ${tokens.accessToken}` };
  }
  if ((request.url ?? "").endsWith("/v1/auth/logout") && tokens?.refreshToken) {
    request.data = JSON.stringify({ refreshToken: tokens.refreshToken });
  }
  return http.request(request);
}
