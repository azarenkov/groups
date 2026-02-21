use async_trait::async_trait;

use domain::{
    room::Room,
    server::{Server, repository::ServerRepository, value_objects::id::ServerId},
    shared::repository::error::RepositoryError,
    user::{User, value_objects::id::UserId},
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::{room::RoomDto, room_type::RoomTypeDto, server::ServerDto, user::UserDto},
    error::map_sqlx_error_to_domain_error,
};

pub struct PostgresServerRepository {
    pool: Pool<Postgres>,
}

impl PostgresServerRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServerRepository for PostgresServerRepository {
    async fn server_exists(&self, server_id: &ServerId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                SELECT EXISTS(SELECT 1 FROM servers WHERE id = $1) as "exists!"
            "#,
            server_id.uuid()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.exists)
    }

    async fn create_server(&self, server: &Server) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
                INSERT INTO servers (id, name, owner_id, description, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            server.id().uuid(),
            server.name().as_str(),
            server.owner_id().uuid(),
            server.description().map(|d| d.as_str()),
            server.created_at(),
            server.updated_at()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
                INSERT INTO server_members (server_id, user_id, joined_at)
                VALUES ($1, $2, NOW())
            "#,
            server.id().uuid(),
            server.owner_id().uuid()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn get_servers_by_user_id(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<Server>, RepositoryError> {
        let servers = sqlx::query_as!(
            ServerDto,
            r#"
                SELECT id, name, owner_id, description, created_at, updated_at
                FROM servers
                WHERE owner_id = $1
                ORDER BY created_at DESC
            "#,
            user_id.uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(servers.into_iter().map(|dto| dto.into()).collect())
    }

    async fn get_users_by_server_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<User>, RepositoryError> {
        let users = sqlx::query_as!(
            UserDto,
            r#"
                SELECT u.id, u.name, u.email, u.password_hash, u.created_at, u.updated_at
                FROM users u
                INNER JOIN server_members sm ON sm.user_id = u.id
                WHERE sm.server_id = $1
                ORDER BY sm.joined_at ASC
            "#,
            server_id.uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(users.into_iter().map(|dto| dto.into()).collect())
    }

    async fn add_member_to_server(
        &self,
        server_id: &ServerId,
        user_id: &UserId,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
                INSERT INTO server_members (server_id, user_id, joined_at)
                VALUES ($1, $2, NOW())
                ON CONFLICT (server_id, user_id) DO NOTHING
            "#,
            server_id.uuid(),
            user_id.uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn get_rooms_by_server_id(
        &self,
        server_id: &ServerId,
    ) -> Result<Vec<Room>, RepositoryError> {
        let rooms = sqlx::query_as!(
            RoomDto,
            r#"
                SELECT
                    id,
                    server_id,
                    owner_id,
                    name,
                    max_participants,
                    room_type as "room_type: RoomTypeDto",
                    password_hash,
                    created_at,
                    updated_at,
                    last_join
                FROM rooms
                WHERE server_id = $1
                ORDER BY created_at DESC
            "#,
            server_id.uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(rooms.into_iter().map(|dto| dto.into()).collect())
    }

    async fn delete_server_by_owner(
        &self,
        server_id: &ServerId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                DELETE FROM servers WHERE id = $1 AND owner_id = $2
            "#,
            server_id.uuid(),
            owner_id.uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.rows_affected() > 0)
    }
}
