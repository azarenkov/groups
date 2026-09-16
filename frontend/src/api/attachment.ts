import { http, UPLOAD_TIMEOUT_MS } from "./http";

import type { Attachment } from "../types/channel";

export const MAX_ATTACHMENT_BYTES = 15 * 1024 * 1024;
export const MAX_ATTACHMENTS_PER_MESSAGE = 10;

class AttachmentAPI {
  private client = http;

  async upload(
    file: File,
    onProgress?: (fraction: number) => void,
  ): Promise<Attachment> {
    const form = new FormData();
    form.append("file", file, file.name);
    const response = await this.client.post("/v1/attachments", form, {
      timeout: UPLOAD_TIMEOUT_MS,
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
      timeout: UPLOAD_TIMEOUT_MS,
    });
    return response.data;
  }
}

export const attachmentAPI = new AttachmentAPI();
