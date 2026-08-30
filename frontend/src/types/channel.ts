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

export interface ChannelParticipant {
  id: string;
  name: string;
  bio?: string;
  profileUpdatedAt?: string | null;
}

export interface ListChannelParticipantsResponse {
  participants: ChannelParticipant[];
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
  muted?: boolean;
  mutedBy?: string | null;
  selfMuted?: boolean;
}

export interface Attachment {
  id: string;
  fileName: string;
  contentType: string;
  sizeBytes: number;
}

export interface ChannelMessage {
  id: string;
  channelId: string;
  authorId: string;
  authorName?: string;
  content: string;
  inviteServerId?: string;
  inviteServerName?: string;
  callStatus?: string;
  callDurationSeconds?: number;
  attachments?: Attachment[];
  createdAt: string;
  editedAt: string | null;
}

export interface ListChannelMessagesResponse {
  messages: ChannelMessage[];
}

export interface SearchChannelMessagesResponse {
  messages: ChannelMessage[];
  pagination?: PaginationMeta;
}
