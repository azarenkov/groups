use chrono::{DateTime, Utc};
use domain::{community::id::CommunityId, user::id::UserId};

const TOKEN_BYTES: usize = 24;
pub const TOKEN_MIN_LEN: usize = 22;
pub const TOKEN_MAX_LEN: usize = 64;
pub const NAME_MAX_LEN: usize = 60;
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteToken(String);

impl InviteToken {
    pub fn generate() -> Self {
        let bytes: [u8; TOKEN_BYTES] = rand::random();
        Self(base64url(&bytes))
    }

    pub fn parse(raw: &str) -> Option<Self> {
        let value = raw.trim().trim_start_matches('+');
        let valid_len = (TOKEN_MIN_LEN..=TOKEN_MAX_LEN).contains(&value.len());
        let valid_chars = value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        (valid_len && valid_chars).then(|| Self(value.to_string()))
    }

    pub fn restore(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(n & 63) as usize] as char);
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteLinkStatus {
    Active,
    Expired,
    Exhausted,
    Revoked,
}

impl InviteLinkStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::Exhausted => "exhausted",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Debug, Clone)]
pub struct InviteLink {
    pub token: InviteToken,
    pub community_id: CommunityId,
    pub created_by: UserId,
    pub creator_name: Option<String>,
    pub name: Option<String>,
    pub is_primary: bool,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl InviteLink {
    pub fn primary(community_id: CommunityId, created_by: UserId, now: DateTime<Utc>) -> Self {
        Self::new(community_id, created_by, None, true, None, None, now)
    }

    pub fn new(
        community_id: CommunityId,
        created_by: UserId,
        name: Option<String>,
        is_primary: bool,
        max_uses: Option<i32>,
        expires_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            token: InviteToken::generate(),
            community_id,
            created_by,
            creator_name: None,
            name,
            is_primary,
            max_uses,
            uses: 0,
            expires_at,
            created_at: now,
            revoked_at: None,
        }
    }

    pub fn status(&self, now: DateTime<Utc>) -> InviteLinkStatus {
        if self.revoked_at.is_some() {
            InviteLinkStatus::Revoked
        } else if self.expires_at.is_some_and(|at| at <= now) {
            InviteLinkStatus::Expired
        } else if self.max_uses.is_some_and(|max| self.uses >= max) {
            InviteLinkStatus::Exhausted
        } else {
            InviteLinkStatus::Active
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InviteJoinOutcome {
    Joined(CommunityId),
    AlreadyMember(CommunityId),
    Unavailable(InviteLinkStatus),
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use uuid::Uuid;

    #[test]
    fn generated_tokens_are_url_safe_and_unique() {
        let a = InviteToken::generate();
        let b = InviteToken::generate();
        assert_ne!(a, b);
        assert_eq!(a.as_str().len(), 32);
        assert!(InviteToken::parse(a.as_str()).is_some());
        assert_eq!(InviteToken::parse(&format!("+{}", a.as_str())), Some(a.clone()));
    }

    #[test]
    fn token_parsing_rejects_short_or_unsafe_values() {
        assert!(InviteToken::parse("short").is_none());
        assert!(InviteToken::parse("abcdefghijklmnopqrstuvwxyz/12345").is_none());
        assert!(InviteToken::parse(&"a".repeat(65)).is_none());
        assert!(InviteToken::parse(&"a".repeat(22)).is_some());
    }

    #[test]
    fn status_reflects_revocation_expiry_and_usage_limits() {
        let now = Utc::now();
        let community = CommunityId::new(Uuid::new_v4());
        let owner = UserId::new(Uuid::new_v4());
        let mut link = InviteLink::new(community, owner, None, false, Some(2), Some(now + Duration::hours(1)), now);
        assert_eq!(link.status(now), InviteLinkStatus::Active);
        link.uses = 2;
        assert_eq!(link.status(now), InviteLinkStatus::Exhausted);
        link.uses = 0;
        assert_eq!(link.status(now + Duration::hours(2)), InviteLinkStatus::Expired);
        link.revoked_at = Some(now);
        assert_eq!(link.status(now), InviteLinkStatus::Revoked);
        let primary = InviteLink::primary(community, owner, now);
        assert!(primary.is_primary);
        assert_eq!(primary.status(now + Duration::days(3650)), InviteLinkStatus::Active);
    }
}
