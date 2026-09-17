import { http } from "../../../shared/api/http";

export interface ReportPayload {
  reason: string;
  details?: string;
  channelId?: string;
  messageId?: string;
}

class ReportAPI {
  private client = http;

  async reportUser(userId: string, payload: ReportPayload): Promise<void> {
    await this.client.post(`/v1/users/${userId}/report`, payload);
  }

  async reportServer(serverId: string, payload: ReportPayload): Promise<void> {
    await this.client.post(`/v1/servers/${serverId}/report`, payload);
  }
}

export const reportAPI = new ReportAPI();
