#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModerationGrant {
    SelfService,
    Moderator,
    Owner,
}

impl ModerationGrant {
    pub fn overrides_others(self) -> bool {
        matches!(self, Self::Moderator | Self::Owner)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Publication {
    Full,
    Restricted,
}
