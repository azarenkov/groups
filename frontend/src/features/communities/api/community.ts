import { http } from "../../../shared/api/http";
import { appendCrop } from "../../../shared/lib/imageCrop";
import type { Permission } from "../lib/permissions";
import { readPermissions, toLegacyMask } from "../lib/permissions";
import {
  ListCommunitiesResponse,
  ListCommunityMembersResponse,
  CreateCommunityRequest,
  CreateCommunityResponse,
  InviteLink,
  InvitePreview,
  JoinInviteResponse,
} from "../../../shared/types/community";

class CommunityAPI {
  private client = http;

  async listCommunities(
    page: number = 1,
    perPage: number = 20,
  ): Promise<ListCommunitiesResponse> {
    const response = await this.client.get("/v1/communities", {
      params: { page, per_page: perPage },
    });
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async createCommunity(data: CreateCommunityRequest): Promise<CreateCommunityResponse> {
    const response = await this.client.post("/v1/communities", data);
    return response.data.data;
  }

  async updateCommunity(
    communityId: string,
    data: { name: string; description?: string },
  ): Promise<void> {
    await this.client.patch(`/v1/communities/${communityId}`, data);
  }

  async uploadLogo(communityId: string, file: File): Promise<void> {
    const form = new FormData();
    form.append("file", file, file.name);
    appendCrop(form, file);
    await this.client.post(`/v1/communities/${communityId}/logo`, form, {
      timeout: 60000,
    });
  }

  async deleteLogo(communityId: string): Promise<void> {
    await this.client.delete(`/v1/communities/${communityId}/logo`);
  }

  async downloadLogo(communityId: string): Promise<Blob> {
    const response = await this.client.get(`/v1/communities/${communityId}/logo`, {
      responseType: "blob",
      timeout: 30000,
    });
    return response.data;
  }

  async listRoles(communityId: string): Promise<{
    roles: { id: string; name: string; color: string; permissions: Permission[]; position: number }[];
    assignments: { userId: string; roleId: string }[];
  }> {
    const response = await this.client.get(`/v1/communities/${communityId}/roles`);
    const data = response.data.data as {
      roles: {
        id: string;
        name: string;
        color: string;
        permissions?: number;
        permissionList?: unknown;
        position: number;
      }[];
      assignments: { userId: string; roleId: string }[];
    };
    return {
      roles: data.roles.map((r) => ({
        id: r.id,
        name: r.name,
        color: r.color,
        permissions: readPermissions(r.permissionList, r.permissions),
        position: r.position,
      })),
      assignments: data.assignments,
    };
  }

  async createRole(
    communityId: string,
    name: string,
    color: string,
    permissions: Permission[],
  ): Promise<{ id: string }> {
    const response = await this.client.post(`/v1/communities/${communityId}/roles`, {
      name,
      color,
      permissions: toLegacyMask(permissions),
      permissionList: permissions,
    });
    return response.data.data;
  }

  async updateRole(
    communityId: string,
    roleId: string,
    patch: {
      name?: string;
      color?: string;
      add?: Permission[];
      remove?: Permission[];
    },
  ): Promise<void> {
    await this.client.put(`/v1/communities/${communityId}/roles/${roleId}`, {
      name: patch.name,
      color: patch.color,
      permissionsAdd: patch.add && patch.add.length > 0 ? toLegacyMask(patch.add) : undefined,
      permissionsRemove:
        patch.remove && patch.remove.length > 0 ? toLegacyMask(patch.remove) : undefined,
      addPermissionList: patch.add && patch.add.length > 0 ? patch.add : undefined,
      removePermissionList: patch.remove && patch.remove.length > 0 ? patch.remove : undefined,
    });
  }

  async deleteRole(communityId: string, roleId: string): Promise<void> {
    await this.client.delete(`/v1/communities/${communityId}/roles/${roleId}`);
  }

  async kickMember(communityId: string, memberId: string): Promise<void> {
    await this.client.delete(`/v1/communities/${communityId}/members/${memberId}`);
  }

  async swapRoles(communityId: string, roleA: string, roleB: string): Promise<void> {
    await this.client.post(`/v1/communities/${communityId}/roles/${roleA}/swap/${roleB}`);
  }

  async assignRole(communityId: string, memberId: string, roleId: string): Promise<void> {
    await this.client.put(
      `/v1/communities/${communityId}/members/${memberId}/roles/${roleId}`,
    );
  }

  async revokeRole(communityId: string, memberId: string, roleId: string): Promise<void> {
    await this.client.delete(
      `/v1/communities/${communityId}/members/${memberId}/roles/${roleId}`,
    );
  }

  async deleteCommunity(communityId: string): Promise<void> {
    await this.client.delete(`/v1/communities/${communityId}`);
  }

  async leave(communityId: string): Promise<void> {
    await this.client.delete(`/v1/communities/${communityId}/members/me`);
  }

  async listMembers(
    communityId: string,
    page: number = 1,
    perPage: number = 50,
  ): Promise<ListCommunityMembersResponse> {
    const response = await this.client.get(
      `/v1/communities/${communityId}/members`,
      { params: { page, per_page: perPage } },
    );
    return {
      ...response.data.data,
      pagination: response.data.meta?.pagination,
    };
  }

  async listAllMembers(communityId: string): Promise<{ id: string; name: string }[]> {
    const perPage = 50;
    const all: { id: string; name: string }[] = [];
    let page = 1;
    while (true) {
      const res = await this.listMembers(communityId, page, perPage);
      all.push(...res.members);
      const total = res.pagination?.totalPages ?? 1;
      if (page >= total) break;
      page++;
    }
    return all;
  }

  async createInviteLink(
    communityId: string,
    opts: { name?: string; expiresDays?: number; maxUses?: number } = {},
  ): Promise<InviteLink> {
    const response = await this.client.post(
      `/v1/communities/${communityId}/invite-links`,
      {
        name: opts.name,
        expiresDays: opts.expiresDays,
        maxUses: opts.maxUses,
      },
    );
    return response.data.data;
  }

  async listInviteLinks(
    communityId: string,
    opts: { cursor?: string | null; limit?: number } = {},
  ): Promise<{ primary: InviteLink; links: InviteLink[]; nextCursor: string | null }> {
    const response = await this.client.get(
      `/v1/communities/${communityId}/invite-links`,
      { params: { cursor: opts.cursor ?? undefined, limit: opts.limit } },
    );
    return response.data.data;
  }

  async revokeInviteLink(communityId: string, token: string): Promise<void> {
    await this.client.delete(
      `/v1/communities/${communityId}/invite-links/${encodeURIComponent(token)}`,
    );
  }

  async previewInvite(token: string): Promise<InvitePreview> {
    const response = await this.client.get(
      `/v1/invites/${encodeURIComponent(token)}`,
    );
    return response.data.data;
  }

  async joinByInvite(token: string): Promise<JoinInviteResponse> {
    const response = await this.client.post(
      `/v1/invites/${encodeURIComponent(token)}/join`,
    );
    return response.data.data;
  }

  async listVoicePresence(communityId: string): Promise<{
    items: {
      channelId: string;
      userId: string;
      userName: string;
      isMuted: boolean;
      mutedBy?: string | null;
      deafenedBy?: string | null;
      cameraOn: boolean;
      screenOn: boolean;
    }[];
    streamViewers: {
      channelId: string;
      streamerId: string;
      count: number;
    }[];
    sessions: { channelId: string; startedAt: string }[];
  }> {
    const response = await this.client.get(
      `/v1/communities/${communityId}/voice-presence`,
    );
    return {
      items: response.data.data.items,
      streamViewers: response.data.data.streamViewers ?? [],
      sessions: response.data.data.sessions ?? [],
    };
  }

  async updateStreamViewer(
    communityId: string,
    channelId: string,
    streamerId: string,
    viewerSessionId: string,
    watching: boolean,
  ): Promise<void> {
    await this.client.put(`/v1/communities/${communityId}/stream-viewers`, {
      channelId,
      streamerId,
      viewerSessionId,
      watching,
    });
  }

  async listSentInvites(communityId: string): Promise<{ recipientIds: string[] }> {
    const response = await this.client.get(
      `/v1/communities/${communityId}/sent-invites`,
    );
    return { recipientIds: response.data.data.recipientIds };
  }
}

export const communityAPI = new CommunityAPI();
