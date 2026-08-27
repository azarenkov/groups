use messaging_domain::friendship::status::FriendshipStatus;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "friendship_status", rename_all = "snake_case")]
pub enum FriendshipStatusDto {
    PendingOutgoing,
    PendingIncoming,
    Accepted,
    Declined,
}

impl From<FriendshipStatus> for FriendshipStatusDto {
    fn from(value: FriendshipStatus) -> Self {
        match value {
            FriendshipStatus::PendingOutgoing => Self::PendingOutgoing,
            FriendshipStatus::PendingIncoming => Self::PendingIncoming,
            FriendshipStatus::Accepted => Self::Accepted,
            FriendshipStatus::Declined => Self::Declined,
        }
    }
}

impl From<FriendshipStatusDto> for FriendshipStatus {
    fn from(value: FriendshipStatusDto) -> Self {
        match value {
            FriendshipStatusDto::PendingOutgoing => Self::PendingOutgoing,
            FriendshipStatusDto::PendingIncoming => Self::PendingIncoming,
            FriendshipStatusDto::Accepted => Self::Accepted,
            FriendshipStatusDto::Declined => Self::Declined,
        }
    }
}
