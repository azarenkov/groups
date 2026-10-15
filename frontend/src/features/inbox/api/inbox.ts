import { http } from "../../../shared/api/http";

export type InboxKind =
  | "community_invitation"
  | "removed_from_community"
  | "friend_request_received"
  | "friend_request_accepted";

export interface InboxItem {
  id: string;
  kind: InboxKind;
  actor: { id: string; name: string };
  community?: { id: string; name: string };
  invitationId?: string;
  createdAt: string;
  read: boolean;
}

export interface InboxPage {
  items: InboxItem[];
  nextCursor: string | null;
}

export const inboxAPI = {
  async list(before?: string | null): Promise<InboxPage> {
    const response = await http.get("/v1/inbox", { params: before ? { before } : undefined });
    return response.data.data;
  },

  async unread(): Promise<number> {
    const response = await http.get("/v1/inbox/unread");
    return response.data.data.count;
  },

  async markRead(ids?: string[]): Promise<void> {
    await http.post("/v1/inbox/read", ids ? { ids } : {});
  },

  async acceptInvitation(invitationId: string): Promise<{ communityId: string }> {
    const response = await http.post(`/v1/invitations/${encodeURIComponent(invitationId)}/accept`);
    return response.data.data;
  },

  async declineInvitation(invitationId: string): Promise<void> {
    await http.post(`/v1/invitations/${encodeURIComponent(invitationId)}/decline`);
  },
};
