import type { Attachment } from "../../shared/types/channel";

export interface ChannelMeta {
  id: string;
  name: string;
  topic?: string;
  type: "text";
}

export interface DmMeta {
  friendId: string;
  name: string;
  initials: string;
  color: string;
  online: boolean;
  isFriend?: boolean;
  friendship?: "accepted" | "pending_outgoing" | "pending_incoming" | "none";
}

export interface PendingFile {
  localId: string;
  file: File;
  progress: number;
  attachment?: Attachment;
  error?: boolean;
  oversize?: boolean;
}
