use async_trait::async_trait;
use domain::user::id::UserId;
use ports::repository::RepositoryError;

pub const PEOPLE_QUERY_MIN_CHARS: usize = 2;
pub const PEOPLE_QUERY_MAX_CHARS: usize = 32;
pub const PEOPLE_LIMIT_DEFAULT: i64 = 8;
pub const PEOPLE_LIMIT_MAX: i64 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonRelation {
    None,
    Friend,
    OutgoingRequest,
    IncomingRequest,
}

impl PersonRelation {
    pub fn from_friendship_status(status: Option<&str>) -> Self {
        match status {
            Some("accepted") => Self::Friend,
            Some("pending_outgoing") => Self::OutgoingRequest,
            Some("pending_incoming") => Self::IncomingRequest,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonMatch {
    pub user_id: UserId,
    pub name: String,
    pub relation: PersonRelation,
    pub accepts_friend_requests: bool,
}

pub fn normalize_people_query(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let chars = trimmed.chars().count();
    (PEOPLE_QUERY_MIN_CHARS..=PEOPLE_QUERY_MAX_CHARS)
        .contains(&chars)
        .then(|| trimmed.to_lowercase())
}

pub fn clamp_people_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(PEOPLE_LIMIT_DEFAULT).clamp(1, PEOPLE_LIMIT_MAX)
}

#[async_trait]
pub trait PeopleDirectory: Send + Sync {
    async fn search(&self, viewer: &UserId, prefix: &str, limit: i64) -> Result<Vec<PersonMatch>, RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_are_trimmed_folded_and_bounded() {
        assert_eq!(normalize_people_query("  AlEx "), Some("alex".to_string()));
        assert_eq!(normalize_people_query("a"), None);
        assert_eq!(normalize_people_query(&"x".repeat(33)), None);
    }

    #[test]
    fn limits_fall_back_and_clamp() {
        assert_eq!(clamp_people_limit(None), PEOPLE_LIMIT_DEFAULT);
        assert_eq!(clamp_people_limit(Some(0)), 1);
        assert_eq!(clamp_people_limit(Some(500)), PEOPLE_LIMIT_MAX);
    }

    #[test]
    fn friendship_rows_map_to_the_viewer_relation() {
        assert_eq!(PersonRelation::from_friendship_status(Some("accepted")), PersonRelation::Friend);
        assert_eq!(PersonRelation::from_friendship_status(Some("pending_outgoing")), PersonRelation::OutgoingRequest);
        assert_eq!(PersonRelation::from_friendship_status(Some("pending_incoming")), PersonRelation::IncomingRequest);
        assert_eq!(PersonRelation::from_friendship_status(Some("declined")), PersonRelation::None);
        assert_eq!(PersonRelation::from_friendship_status(None), PersonRelation::None);
    }
}
