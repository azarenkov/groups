import { http } from "./http";

import {
  FriendshipStatus,
  FriendIdsResponse,
  ListFriendshipsResponse,
  ListBlocksResponse,
  BlockIdsResponse,
  BlockRelation,
} from "../types/friendship";

class FriendshipAPI {
  private client = http;

  async sendRequest(username: string): Promise<void> {
    await this.client.post("/v1/users/me/friends", { username });
  }

  async accept(friendId: string): Promise<void> {
    await this.client.post(`/v1/users/me/friends/${friendId}/accept`);
  }

  async remove(friendId: string): Promise<void> {
    await this.client.delete(`/v1/users/me/friends/${friendId}`);
  }

  async list(
    opts: { status?: FriendshipStatus; cursor?: string | null; limit?: number } = {},
  ): Promise<ListFriendshipsResponse> {
    const response = await this.client.get("/v1/users/me/friends", {
      params: {
        status: opts.status,
        cursor: opts.cursor ?? undefined,
        limit: opts.limit,
      },
    });
    return response.data.data;
  }

  async listIds(): Promise<FriendIdsResponse> {
    const response = await this.client.get("/v1/users/me/friends/ids");
    return response.data.data;
  }

  async block(userId: string): Promise<{ occurredAt: string }> {
    const response = await this.client.post(`/v1/users/${userId}/block`);
    return response.data.data;
  }

  async unblock(userId: string): Promise<{ occurredAt: string }> {
    const response = await this.client.delete(`/v1/users/${userId}/block`);
    return response.data.data;
  }

  async blockRelation(userId: string): Promise<BlockRelation> {
    const response = await this.client.get(`/v1/users/${userId}/block`);
    return response.data.data;
  }

  async listBlocks(
    opts: { cursor?: string | null; limit?: number } = {},
  ): Promise<ListBlocksResponse> {
    const response = await this.client.get("/v1/users/me/blocks", {
      params: { cursor: opts.cursor ?? undefined, limit: opts.limit },
    });
    return response.data.data;
  }

  async listBlockIds(): Promise<BlockIdsResponse> {
    const response = await this.client.get("/v1/users/me/blocks/ids");
    return response.data.data;
  }

  async searchUsers(
    query: string,
    limit: number = 8,
  ): Promise<{ id: string; name: string }[]> {
    const response = await this.client.get("/v1/users/search", {
      params: { q: query, limit },
    });
    return response.data.data.users;
  }
}

export const friendshipAPI = new FriendshipAPI();
