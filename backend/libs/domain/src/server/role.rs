use chrono::{DateTime, Utc};
use uuid::Uuid;

pub const PERM_MANAGE_CHANNELS: i64 = 1;
pub const PERM_MANAGE_INVITES: i64 = 2;
pub const PERM_MANAGE_MESSAGES: i64 = 4;
pub const PERM_MUTE_MEMBERS: i64 = 8;
pub const PERM_MANAGE_ROLES: i64 = 16;
pub const PERM_MANAGE_SERVER: i64 = 32;
pub const PERM_KICK_MEMBERS: i64 = 64;

pub const PERM_ALL: i64 = PERM_MANAGE_CHANNELS
    | PERM_MANAGE_INVITES
    | PERM_MANAGE_MESSAGES
    | PERM_MUTE_MEMBERS
    | PERM_MANAGE_ROLES
    | PERM_MANAGE_SERVER
    | PERM_KICK_MEMBERS;

#[derive(Debug, Clone)]
pub struct ServerRole {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub color: String,
    pub permissions: i64,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}

pub fn has_permission(permissions: i64, perm: i64) -> bool {
    permissions & perm != 0
}

#[derive(Debug, Clone, Default)]
pub struct RolePatch {
    pub name: Option<String>,
    pub color: Option<String>,
    pub add_permissions: i64,
    pub remove_permissions: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOutcome {
    Assigned,
    NotFound,
    Denied,
}

#[derive(Debug, Clone)]
pub enum CreateOutcome {
    Created(ServerRole),
    LimitReached,
    Denied,
}

#[async_trait::async_trait]
pub trait ServerRoleRepository {
    async fn list_roles(
        &self,
        server_id: &crate::server::id::ServerId,
    ) -> Result<Vec<ServerRole>, crate::shared::repository::error::RepositoryError>;

    async fn create_role(
        &self,
        role: &ServerRole,
        delegated_actor: Option<&crate::user::id::UserId>,
        max_roles: usize,
        events: &[crate::shared::outbox::outbox_record::OutboxRecord],
    ) -> Result<CreateOutcome, crate::shared::repository::error::RepositoryError>;

    async fn update_role(
        &self,
        server_id: &crate::server::id::ServerId,
        role_id: Uuid,
        patch: &RolePatch,
        delegated_actor: Option<&crate::user::id::UserId>,
        events: &[crate::shared::outbox::outbox_record::OutboxRecord],
    ) -> Result<AssignOutcome, crate::shared::repository::error::RepositoryError>;

    async fn delete_role(
        &self,
        server_id: &crate::server::id::ServerId,
        role_id: Uuid,
        delegated_actor: Option<&crate::user::id::UserId>,
        events: &[crate::shared::outbox::outbox_record::OutboxRecord],
    ) -> Result<AssignOutcome, crate::shared::repository::error::RepositoryError>;

    async fn assign_role(
        &self,
        server_id: &crate::server::id::ServerId,
        user_id: &crate::user::id::UserId,
        role_id: Uuid,
        delegated_actor: Option<&crate::user::id::UserId>,
        events: &[crate::shared::outbox::outbox_record::OutboxRecord],
    ) -> Result<AssignOutcome, crate::shared::repository::error::RepositoryError>;

    async fn revoke_role(
        &self,
        server_id: &crate::server::id::ServerId,
        user_id: &crate::user::id::UserId,
        role_id: Uuid,
        delegated_actor: Option<&crate::user::id::UserId>,
        events: &[crate::shared::outbox::outbox_record::OutboxRecord],
    ) -> Result<AssignOutcome, crate::shared::repository::error::RepositoryError>;

    async fn swap_positions(
        &self,
        server_id: &crate::server::id::ServerId,
        role_a: Uuid,
        role_b: Uuid,
        delegated_actor: Option<&crate::user::id::UserId>,
        events: &[crate::shared::outbox::outbox_record::OutboxRecord],
    ) -> Result<AssignOutcome, crate::shared::repository::error::RepositoryError>;

    async fn member_role_ids(
        &self,
        server_id: &crate::server::id::ServerId,
    ) -> Result<Vec<(Uuid, Uuid)>, crate::shared::repository::error::RepositoryError>;

    async fn user_permissions(
        &self,
        server_id: &crate::server::id::ServerId,
        user_id: &crate::user::id::UserId,
    ) -> Result<i64, crate::shared::repository::error::RepositoryError>;
}
