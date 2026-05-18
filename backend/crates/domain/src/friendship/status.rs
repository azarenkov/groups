#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendshipStatus {
    PendingOutgoing,
    PendingIncoming,
    Accepted,
    Declined,
}
