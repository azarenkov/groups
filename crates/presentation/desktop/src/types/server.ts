export interface Server {
  id: string;
  name: string;
  ownerId: string;
  description: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface PaginationMeta {
  total: number;
  page: number;
  perPage: number;
  totalPages: number;
}

export interface ListServersResponse {
  servers: Server[];
  pagination: PaginationMeta;
}

export interface CreateServerRequest {
  name: string;
  description?: string;
}

export interface CreateServerResponse {
  id: string;
  name: string;
  ownerId: string;
  description: string | null;
  createdAt: string;
  updatedAt: string;
}
