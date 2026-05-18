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
}
