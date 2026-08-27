pub mod assign_role;
pub mod create_role;
pub mod delete_role;
pub mod list_roles;
pub mod revoke_role;
pub mod swap_roles;
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
    #[error("Role is not below your highest role")]
    HierarchyViolation,
    #[error("Role not found")]
    RoleNotFound,
    #[error("Invalid role data")]
    InvalidData,
}

pub const MAX_ROLE_NAME_LEN: usize = 40;
pub const MAX_ROLES_PER_SERVER: usize = 50;

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

pub enum ActorHeight {
    Owner,
    Position(i32),
}

pub async fn actor_height<S, R>(
    server_repository: &S,
    role_repository: &R,
    server_id: &ServerId,
    user_id: &UserId,
) -> Result<ActorHeight, RoleError>
where
    S: ServerRepository,
    R: role::ServerRoleRepository,
{
    let owner = server_repository.find_server_owner(server_id).await?;
    if owner.as_ref() == Some(user_id) {
        return Ok(ActorHeight::Owner);
    }
    let roles = role_repository.list_roles(server_id).await?;
    let assignments = role_repository.member_role_ids(server_id).await?;
    let mine: Vec<Uuid> = assignments
        .into_iter()
        .filter(|(uid, _)| *uid == user_id.uuid())
        .map(|(_, rid)| rid)
        .collect();
    let top = roles
        .iter()
        .filter(|r| mine.contains(&r.id))
        .map(|r| r.position)
        .max()
        .unwrap_or(i32::MIN);
    Ok(ActorHeight::Position(top))
}

pub fn ensure_below(height: &ActorHeight, role_position: i32) -> Result<(), RoleError> {
    match height {
        ActorHeight::Owner => Ok(()),
        ActorHeight::Position(top) if role_position < *top => Ok(()),
        _ => Err(RoleError::HierarchyViolation),
    }
}

pub async fn actor_grantable_permissions<S, R>(
    server_repository: &S,
    role_repository: &R,
    server_id: &ServerId,
    user_id: &UserId,
) -> Result<i64, RoleError>
where
    S: ServerRepository,
    R: role::ServerRoleRepository,
{
    let owner = server_repository.find_server_owner(server_id).await?;
    if owner.as_ref() == Some(user_id) {
        return Ok(role::PERM_ALL);
    }
    Ok(role_repository.user_permissions(server_id, user_id).await?)
}

pub fn ensure_grantable(requested: i64, grantable: i64) -> Result<(), RoleError> {
    if requested & !grantable != 0 {
        return Err(RoleError::NotAllowed);
    }
    Ok(())
}
