export enum RoomType {
  Public = "public",
  Private = "private",
}

export interface Room {
  id: string;
  name: string;
  ownerId: string;
  roomType: RoomType;
  maxParticipants: number;
  createdAt: string;
}

export interface PaginationMeta {
  total: number;
  page: number;
  perPage: number;
  totalPages: number;
}

export interface ListRoomsResponse {
  rooms: Room[];
  pagination: PaginationMeta;
}

export interface CreateRoomRequest {
  serverId: string;
  name: string;
  roomType: RoomType;
  password?: string;
}

export interface CreateRoomResponse {
  id: string;
  name: string;
  ownerId: string;
  roomType: RoomType;
  maxParticipants: number;
  createdAt: string;
}

export interface JoinRoomRequest {
  roomId: string;
  serverId: string;
}

export interface JoinRoomResponse {
  token: string;
  url: string;
}
