import { http, API_BASE_URL } from "../../../shared/api/http";

export interface CallParticipant {
  userId: string;
  userName: string;
}

export interface CallToken {
  token: string;
  url: string;
  participants: CallParticipant[];
  video?: boolean;
}

class CallAPI {
  private client = http;

  async start(
    recipientIds: string[],
    video = false,
  ): Promise<{ callId: string } & CallToken> {
    const response = await this.client.post("/v1/calls", { recipientIds, video });
    return response.data.data;
  }

  async accept(callId: string): Promise<CallToken> {
    const response = await this.client.post(`/v1/calls/${callId}/accept`, {});
    return response.data.data;
  }

  async getActive(): Promise<
    ({ callId: string; startedAt?: string | null } & CallToken) | null
  > {
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

  async leaveWithToken(callId: string, accessToken: string): Promise<void> {
    await fetch(`${API_BASE_URL}/v1/calls/${callId}/leave`, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json",
      },
      body: "{}",
    });
  }

  async addParticipants(callId: string, recipientIds: string[]): Promise<void> {
    await this.client.post(`/v1/calls/${callId}/participants`, { recipientIds });
  }
}

export const callAPI = new CallAPI();
