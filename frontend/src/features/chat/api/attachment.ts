import { http, UPLOAD_TIMEOUT_MS } from "../../../shared/api/http";

import type { Attachment } from "../../../shared/types/channel";

import {
  ATTACHMENTS_PER_MESSAGE,
  MAX_ATTACHMENT_BYTES_BY_TIER,
} from "../../subscription/lib/plans";
import { useEntitlementsStore } from "../../subscription/store/entitlementsStore";

const SLOWEST_UPLOAD_BYTES_PER_MS = 128;
const SLOWEST_DOWNLOAD_BYTES_PER_MS = 128;

export const transferTimeoutMs = (sizeBytes: number, bytesPerMs: number) =>
  UPLOAD_TIMEOUT_MS + Math.ceil(Math.max(0, sizeBytes) / bytesPerMs);

export const maxAttachmentBytes = () =>
  MAX_ATTACHMENT_BYTES_BY_TIER[useEntitlementsStore.getState().tier];

export const maxAttachmentsPerMessage = () =>
  ATTACHMENTS_PER_MESSAGE[useEntitlementsStore.getState().tier];

class AttachmentAPI {
  private client = http;

  async upload(
    file: File,
    onProgress?: (fraction: number) => void,
  ): Promise<Attachment> {
    const form = new FormData();
    form.append("file", file, file.name);
    const response = await this.client.post("/v1/attachments", form, {
      timeout: transferTimeoutMs(file.size, SLOWEST_UPLOAD_BYTES_PER_MS),
      onUploadProgress: (e) => {
        if (onProgress && e.total) onProgress(e.loaded / e.total);
      },
    });
    return response.data.data;
  }

  async delete(attachmentId: string): Promise<void> {
    await this.client.delete(`/v1/attachments/${attachmentId}`);
  }

  async download(attachmentId: string, sizeBytes = 0): Promise<Blob> {
    const response = await this.client.get(`/v1/attachments/${attachmentId}`, {
      responseType: "blob",
      timeout: transferTimeoutMs(sizeBytes, SLOWEST_DOWNLOAD_BYTES_PER_MS),
    });
    return response.data;
  }
}

export const attachmentAPI = new AttachmentAPI();
