export type FriendshipStatus =
  | "pending_outgoing"
  | "pending_incoming"
  | "accepted"
  | "declined";

export interface Friendship {
  friendId: string;
  friendName: string;
  status: FriendshipStatus;
  createdAt: string;
  updatedAt: string;
}

export interface ListFriendshipsResponse {
  friends: Friendship[];
  nextCursor: string | null;
}

export interface FriendIdsResponse {
  friends: { friendId: string; status: FriendshipStatus }[];
}

export interface BlockedUser {
  userId: string;
  name: string;
  createdAt: string;
}

export interface BlockRelation {
  blocked: boolean;
  blockedBy: boolean;
}
