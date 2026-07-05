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

export interface InviteCode {
  code: string;
  createdBy: string;
  creatorName: string | null;
  uses: number;
  maxUses: number | null;
  expiresAt: string | null;
  createdAt: string;
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
