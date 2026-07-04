pub mod repository;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallParticipantStatus {
    Invited,
    Joined,
    Declined,
    Left,
}

impl CallParticipantStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Invited => "invited",
            Self::Joined => "joined",
            Self::Declined => "declined",
            Self::Left => "left",
        }
    }
}
