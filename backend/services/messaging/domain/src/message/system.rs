use domain::user::id::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    User,
    System,
}

impl MessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMessageKind {
    MemberJoined,
    MemberLeft,
}

impl SystemMessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MemberJoined => "member_joined",
            Self::MemberLeft => "member_left",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "member_joined" => Some(Self::MemberJoined),
            "member_left" => Some(Self::MemberLeft),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemMessage {
    pub kind: SystemMessageKind,
    pub user_id: UserId,
}

impl SystemMessage {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "type": self.kind.as_str(), "userId": self.user_id.uuid() })
    }

    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        let kind = SystemMessageKind::parse(value.get("type")?.as_str()?)?;
        let user_id = value.get("userId")?.as_str()?.parse().ok()?;
        Some(Self {
            kind,
            user_id: UserId::new(user_id),
        })
    }
}
