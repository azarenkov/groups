use async_trait::async_trait;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeopleQuery {
    Id(UserId),
    Name(String),
}

impl PeopleQuery {
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if let Ok(id) = Uuid::parse_str(trimmed) {
            return Some(Self::Id(UserId::new(id)));
        }
        let chars = trimmed.chars().count();
        (PEOPLE_QUERY_MIN_CHARS..=PEOPLE_QUERY_MAX_CHARS)
            .contains(&chars)
            .then(|| Self::Name(trimmed.to_lowercase()))
    }
}

pub fn clamp_people_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(PEOPLE_LIMIT_DEFAULT).clamp(1, PEOPLE_LIMIT_MAX)
}

#[async_trait]
pub trait PeopleDirectory: Send + Sync {
    async fn search(&self, viewer: &UserId, query: &PeopleQuery, limit: i64) -> Result<Vec<PersonMatch>, RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_queries_are_trimmed_folded_and_bounded() {
        assert_eq!(PeopleQuery::parse("  AlEx "), Some(PeopleQuery::Name("alex".to_string())));
        assert_eq!(PeopleQuery::parse("a"), None);
        assert_eq!(PeopleQuery::parse(&"x".repeat(33)), None);
    }

    #[test]
    fn a_user_id_looks_up_that_person() {
        let id = Uuid::new_v4();
        assert_eq!(PeopleQuery::parse(&format!(" {} ", id.to_string().to_uppercase())), Some(PeopleQuery::Id(UserId::new(id))));
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
