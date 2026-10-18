use chrono::{DateTime, Utc};
use domain::{community::id::CommunityId, user::id::UserId};
use messaging_domain::community::invite_link::{InviteLink, InviteToken};
use uuid::Uuid;

#[derive(Debug)]
pub struct InviteLinkRow {
    pub token: String,
    pub community_id: Uuid,
    pub created_by: Uuid,
    pub creator_name: Option<String>,
    pub name: Option<String>,
    pub is_primary: bool,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl From<InviteLinkRow> for InviteLink {
    fn from(row: InviteLinkRow) -> Self {
        InviteLink {
            token: InviteToken::restore(row.token),
            community_id: CommunityId::new(row.community_id),
            created_by: UserId::new(row.created_by),
            creator_name: row.creator_name,
            name: row.name,
            is_primary: row.is_primary,
            max_uses: row.max_uses,
            uses: row.uses,
            expires_at: row.expires_at,
            created_at: row.created_at,
            revoked_at: row.revoked_at,
        }
    }
}
