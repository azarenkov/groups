use uuid::Uuid;

use crate::{
    server::{authorization::AuthorizationSnapshot, id::ServerId, role::ServerRole},
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
};

#[async_trait::async_trait]
pub trait ServerUnitOfWork: Send + Sync {
    async fn begin(&self, server_id: &ServerId) -> Result<Box<dyn ServerTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait ServerTransaction: Send {
    async fn authorization(&mut self) -> Result<AuthorizationSnapshot, RepositoryError>;
    async fn insert_role(&mut self, role: &ServerRole) -> Result<(), RepositoryError>;
    async fn update_role(
        &mut self,
        role_id: Uuid,
        name: Option<&str>,
        color: Option<&str>,
        permissions: i64,
    ) -> Result<bool, RepositoryError>;
    async fn delete_role(&mut self, role_id: Uuid) -> Result<bool, RepositoryError>;
    async fn assign_role(&mut self, user_id: &UserId, role_id: Uuid) -> Result<(), RepositoryError>;
    async fn revoke_role(&mut self, user_id: &UserId, role_id: Uuid) -> Result<bool, RepositoryError>;
    async fn swap_role_positions(&mut self, role_a: Uuid, role_b: Uuid) -> Result<bool, RepositoryError>;
    async fn save_outbox(&mut self, records: &[OutboxRecord]) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
