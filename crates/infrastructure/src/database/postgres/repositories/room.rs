use async_trait::async_trait;

use domain::{
    room::{
        Room,
        repository::RoomRepository,
        value_objects::{id::RoomId, name::RoomName},
    },
    shared::repository::error::RepositoryError,
    user::value_objects::id::UserId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::room_type::RoomTypeDto, error::map_sqlx_error_to_domain_error,
};

pub struct PostgresRoomRepository {
    pool: Pool<Postgres>,
}

impl PostgresRoomRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoomRepository for PostgresRoomRepository {
    async fn create_room(&self, room: &Room) -> Result<(), RepositoryError> {
        let room_type_dto: RoomTypeDto = room.room_type().into();

        sqlx::query!(
            r#"
                INSERT INTO rooms (id, server_id, owner_id, name, max_participants, room_type, password_hash, created_at, updated_at, last_join)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            room.id().uuid(),
            room.server_id().uuid(),
            room.owner_id().uuid(),
            room.name().as_str(),
            room.max_participants().value() as i16,
            room_type_dto as RoomTypeDto,
            room.password_hash().map(|ph| ph.as_str()),
            room.created_at(),
            room.updated_at(),
            room.last_join()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(())
    }

    async fn get_room_name_by_id(&self, room_id: &RoomId) -> Result<RoomName, RepositoryError> {
        let record = sqlx::query!(
            r#"
                 SELECT name
                 FROM rooms
                 WHERE id = $1
             "#,
            room_id.uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        match record {
            Some(row) => Ok(RoomName::new(row.name)),
            None => Err(RepositoryError::NotFound),
        }
    }

    async fn delete_room_by_owner(
        &self,
        room_id: &RoomId,
        owner_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
                DELETE FROM rooms WHERE id = $1 AND owner_id = $2
            "#,
            room_id.uuid(),
            owner_id.uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        Ok(result.rows_affected() > 0)
    }
}
