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
  isDefault?: boolean;
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
  isDefault?: boolean;
}

export type SystemMessageType = "member_joined" | "member_left";

export interface SystemMessageInfo {
  type: SystemMessageType;
  userId: string;
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
  deafenedBy?: string | null;
}

export interface VoiceMeta {
  durationMs: number;
  waveform: number[];
}

export interface VideoNoteMeta {
  durationMs: number;
  size: number;
  poster: string;
}

export interface Attachment {
  id: string;
  fileName: string;
  contentType: string;
  sizeBytes: number;
  voice?: VoiceMeta;
  videoNote?: VideoNoteMeta;
}

export interface StickerSnapshot {
  id: string;
  packId: string;
  emoji: string;
  animated: boolean;
  width: number;
  height: number;
}

export interface ReplyPreview {
  id: string;
  authorId?: string;
  authorName?: string;
  content: string;
  hasAttachments: boolean;
  voiceDurationMs?: number | null;
  videoNoteDurationMs?: number | null;
  videoNotePoster?: string | null;
  sticker?: StickerSnapshot | null;
  deleted: boolean;
}

export type ForwardSource =
  | { kind: "channel"; communityId: string; channelId: string }
  | { kind: "dm" };

export interface ForwardInfo {
  messageId: string;
  authorId: string;
  authorName: string;
  sentAt: string;
  source: ForwardSource;
}

export interface ChannelMessage {
  id: string;
  channelId: string;
  authorId: string;
  authorName?: string;
  content: string;
  inviteCommunityId?: string;
  inviteCommunityName?: string;
  callStatus?: string;
  callDurationSeconds?: number;
  attachments?: Attachment[];
  kind?: "user" | "system";
  system?: SystemMessageInfo;
  replyTo?: ReplyPreview;
  forwardedFrom?: ForwardInfo | null;
  sticker?: StickerSnapshot | null;
  voiceListened?: boolean;
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
