pub mod assign_role;
pub mod create_role;
pub mod delete_role;
pub mod list_roles;
pub mod revoke_role;
pub mod swap_roles;
pub mod update_role;

use chrono::Utc;
use messaging_domain::{server::authorization::AuthorizationSnapshot, server::id::ServerId, server::policy::PolicyError, server::repository::ServerRepository};
use domain::{shared::outbox::outbox_record::AggregateType, shared::outbox::outbox_record::OutboxRecord, shared::repository::error::RepositoryError, user::id::UserId};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the server")]
    NotMember,
    #[error("Missing permission to manage roles")]
    NotAllowed,
    #[error("Role is not below your highest role")]
    HierarchyViolation,
    #[error("Role not found")]
    RoleNotFound,
    #[error("Invalid role data")]
    InvalidData,
}

impl From<PolicyError> for RoleError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => RoleError::NotMember,
            PolicyError::NotAllowed => RoleError::NotAllowed,
            PolicyError::HierarchyViolation => RoleError::HierarchyViolation,
            PolicyError::RoleNotFound | PolicyError::TargetNotMember => RoleError::RoleNotFound,
            PolicyError::LimitReached => RoleError::InvalidData,
        }
    }
}

pub const MAX_ROLE_NAME_LEN: usize = 40;

pub fn validate_role_input(name: &str, color: &str) -> Result<(), RoleError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_ROLE_NAME_LEN {
        return Err(RoleError::InvalidData);
    }
    let ok_color = color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|b| b.is_ascii_hexdigit());
    if !ok_color {
        return Err(RoleError::InvalidData);
    }
    Ok(())
}

pub async fn ensure_member<S: ServerRepository>(
    server_repository: &S,
    server_id: &ServerId,
    user_id: &UserId,
) -> Result<(), RoleError> {
    if !server_repository.is_server_member(server_id, user_id).await? {
        return Err(RoleError::NotMember);
    }
    Ok(())
}

pub fn roles_changed_event(server_id: &ServerId, snapshot: &AuthorizationSnapshot) -> OutboxRecord {
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: server_id.uuid(),
        aggregate_type: AggregateType::Server,
        parent_id: None,
        parent_type: None,
        event_type: "server.roles.changed".to_string(),
        payload: serde_json::json!({
            "server_id": server_id.uuid(),
            "recipients": snapshot.members.iter().map(|u| u.uuid()).collect::<Vec<_>>(),
        }),
        occurred_at: Utc::now(),
        request_id: None,
    }
}
