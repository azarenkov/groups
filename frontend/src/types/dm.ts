export type DirectFriendship =
  | "accepted"
  | "pending_outgoing"
  | "pending_incoming"
  | "none";

export interface DirectConversation {
  id: string;
  name: string;
  initials: string;
  color: string;
  online: boolean;
  unread: number;
  lastMessage?: string;
  isFriend?: boolean;
  friendship?: DirectFriendship;
}
