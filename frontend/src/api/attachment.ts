import axios, { AxiosError, AxiosInstance, AxiosRequestConfig } from "axios";

import { useAuthStore } from "../store/authStore";
import { toApiError } from "../utils/apiError";
import { rateLimitRetry } from "../utils/rateLimitRetry";
import type { Attachment } from "../types/channel";

const API_BASE_URL =
  process.env.REACT_APP_API_URL || "http://localhost:8080/api";

export const MAX_ATTACHMENT_BYTES = 15 * 1024 * 1024;
export const MAX_ATTACHMENTS_PER_MESSAGE = 10;

class AttachmentAPI {
  private client: AxiosInstance;

  constructor() {
    this.client = axios.create({
      baseURL: API_BASE_URL,
      timeout: 120000,
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
        const retried = rateLimitRetry(this.client, error);
        if (retried) return retried;
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

  async upload(
    file: File,
    onProgress?: (fraction: number) => void,
  ): Promise<Attachment> {
    const form = new FormData();
    form.append("file", file, file.name);
    const response = await this.client.post("/v1/attachments", form, {
      onUploadProgress: (e) => {
        if (onProgress && e.total) onProgress(e.loaded / e.total);
      },
    });
    return response.data.data;
  }

  async delete(attachmentId: string): Promise<void> {
    await this.client.delete(`/v1/attachments/${attachmentId}`);
  }

  async download(attachmentId: string): Promise<Blob> {
    const response = await this.client.get(`/v1/attachments/${attachmentId}`, {
      responseType: "blob",
    });
    return response.data;
  }
}

export const attachmentAPI = new AttachmentAPI();
