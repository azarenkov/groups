pub mod assign_role;
pub mod create_role;
pub mod delete_role;
pub mod list_roles;
pub mod revoke_role;
pub mod update_role;

use chrono::Utc;
use domain::{
    server::{id::ServerId, repository::ServerRepository, role},
    shared::{outbox::outbox_record::{AggregateType, OutboxRecord}, repository::error::RepositoryError},
    user::id::UserId,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the server")]
    NotMember,
    #[error("Missing permission to manage roles")]
    NotAllowed,
    #[error("Role not found")]
    RoleNotFound,
    #[error("Invalid role data")]
    InvalidData,
}

pub const MAX_ROLE_NAME_LEN: usize = 40;
pub const MAX_ROLES_PER_SERVER: usize = 50;

pub fn validate_role_input(name: &str, color: &str) -> Result<(), RoleError> {
    let name = name.trim();
    if name.is_empty() || name.len() > MAX_ROLE_NAME_LEN {
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

pub async fn ensure_manage_roles<S, R>(
    server_repository: &S,
    role_repository: &R,
    server_id: &ServerId,
    user_id: &UserId,
) -> Result<(), RoleError>
where
    S: ServerRepository,
    R: role::ServerRoleRepository,
{
    ensure_member(server_repository, server_id, user_id).await?;
    let owner = server_repository.find_server_owner(server_id).await?;
    if owner.as_ref() == Some(user_id) {
        return Ok(());
    }
    let perms = role_repository.user_permissions(server_id, user_id).await?;
    if role::has_permission(perms, role::PERM_MANAGE_ROLES) {
        return Ok(());
    }
    Err(RoleError::NotAllowed)
}

pub fn roles_changed_event(server_id: &ServerId, recipients: &[UserId]) -> OutboxRecord {
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: server_id.uuid(),
        aggregate_type: AggregateType::Server,
        parent_id: None,
        parent_type: None,
        event_type: "server.roles.changed".to_string(),
        payload: serde_json::json!({
            "server_id": server_id.uuid(),
            "recipients": recipients.iter().map(|u| u.uuid()).collect::<Vec<_>>(),
        }),
        occurred_at: Utc::now(),
        request_id: None,
    }
}

pub async fn recipients<S: ServerRepository>(
    server_repository: &S,
    server_id: &ServerId,
) -> Result<Vec<UserId>, RoleError> {
    Ok(server_repository.find_member_ids_by_server_id(server_id).await?)
}
