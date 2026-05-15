export type ChannelKind = "text" | "voice";
export type ChannelVisibility = "public" | "private";

export interface Channel {
  id: string;
  name: string;
  ownerId: string;
  kind: ChannelKind;
  visibility: ChannelVisibility;
  maxParticipants: number;
  createdAt: string;
}

export interface PaginationMeta {
  total: number;
  page: number;
  perPage: number;
  totalPages: number;
}

export interface ListChannelsResponse {
  channels: Channel[];
  pagination?: PaginationMeta;
}

export interface CreateChannelRequest {
  name: string;
  kind: ChannelKind;
  visibility: ChannelVisibility;
  password?: string;
}

export interface CreateChannelResponse {
  id: string;
  name: string;
  ownerId: string;
  kind: ChannelKind;
  visibility: ChannelVisibility;
  maxParticipants: number;
  createdAt: string;
}

export interface JoinVoiceChannelRequest {
  password?: string;
}

export interface JoinVoiceChannelResponse {
  token: string;
  url: string;
}
