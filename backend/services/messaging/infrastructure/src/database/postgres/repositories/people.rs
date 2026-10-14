use async_trait::async_trait;
use domain::user::id::UserId;
use messaging_domain::people::{PeopleDirectory, PersonMatch, PersonRelation};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresPeopleDirectory {
    pool: Pool<Postgres>,
}

impl PostgresPeopleDirectory {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

fn prefix_pattern(prefix: &str) -> String {
    let escaped = prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("{escaped}%")
}

#[async_trait]
impl PeopleDirectory for PostgresPeopleDirectory {
    async fn search(&self, viewer: &UserId, prefix: &str, limit: i64) -> Result<Vec<PersonMatch>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT p.user_id, p.name, f.status::text AS "status?",
                   COALESCE(s.accept_friend_requests, true) AS "accepts_friend_requests!"
            FROM messaging_user_profiles p
            LEFT JOIN friendships f ON f.user_id = $1 AND f.friend_id = p.user_id
            LEFT JOIN user_privacy_settings s ON s.user_id = p.user_id
            WHERE lower(p.name) LIKE $2 ESCAPE '\'
              AND p.active
              AND p.name <> ''
              AND p.user_id <> $1
              AND NOT EXISTS (SELECT 1 FROM messaging_deleted_users d WHERE d.user_id = p.user_id)
              AND NOT EXISTS (
                  SELECT 1 FROM user_blocks b
                  WHERE (b.blocker_id = $1 AND b.blocked_id = p.user_id)
                     OR (b.blocker_id = p.user_id AND b.blocked_id = $1)
              )
            ORDER BY lower(p.name) = $3 DESC, f.status = 'accepted' DESC NULLS LAST, lower(p.name), p.user_id
            LIMIT $4
            "#,
            viewer.uuid(),
            prefix_pattern(prefix),
            prefix,
            limit,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| PersonMatch {
                user_id: UserId::new(r.user_id),
                name: r.name,
                relation: PersonRelation::from_friendship_status(r.status.as_deref()),
                accepts_friend_requests: r.accepts_friend_requests,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::prefix_pattern;

    #[test]
    fn like_wildcards_in_the_query_match_literally() {
        assert_eq!(prefix_pattern("a_b%c\\"), "a\\_b\\%c\\\\%");
    }
}
