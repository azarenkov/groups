export interface Server {
  id: string;
  name: string;
  ownerId: string;
  description: string | null;
  accentColor: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Member {
  id: string;
  name: string;
}

export interface InviteLink {
  token: string;
  url: string;
  name: string | null;
  isPrimary: boolean;
  createdBy: string;
  creatorName: string | null;
  uses: number;
  maxUses: number | null;
  expiresAt: string | null;
  createdAt: string;
}

export type InviteStatus = "active" | "expired" | "exhausted" | "revoked";

export interface InvitePreview {
  status: InviteStatus;
  server: {
    id: string;
    name: string;
    description: string | null;
    accentColor: string | null;
    memberCount: number;
  } | null;
}

export interface JoinInviteResponse {
  server: Server;
  alreadyMember: boolean;
}

export interface PaginationMeta {
  total: number;
  page: number;
  perPage: number;
  totalPages: number;
}

export interface ListServersResponse {
  servers: Server[];
  pagination?: PaginationMeta;
}

export interface ListServerMembersResponse {
  members: Member[];
  pagination?: PaginationMeta;
}

export interface CreateServerRequest {
  name: string;
  description?: string;
  accentColor?: string;
}

export interface CreateServerResponse {
  id: string;
  name: string;
  ownerId: string;
  description: string | null;
  accentColor: string | null;
  createdAt: string;
  updatedAt: string;
}
