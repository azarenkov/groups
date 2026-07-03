use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{
    call::{repository::CallRepository, CallParticipantStatus},
    shared::{
        outbox::outbox_record::{AggregateType, OutboxRecord},
        repository::error::RepositoryError,
    },
    user::id::UserId,
};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub struct PostgresCallRepository {
    pool: Pool<Postgres>,
}

impl PostgresCallRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

fn call_event(
    call_id: Uuid,
    event_type: &str,
    actor_id: Uuid,
    recipients: &[Uuid],
    request_id: Option<String>,
    occurred_at: DateTime<Utc>,
) -> OutboxRecord {
    OutboxRecord {
        id: Uuid::new_v4(),
        aggregate_id: call_id,
        aggregate_type: AggregateType::User,
        parent_id: None,
        parent_type: None,
        event_type: event_type.to_string(),
        payload: serde_json::json!({
            "call_id": call_id,
            "actor_id": actor_id,
            "recipients": recipients,
        }),
        occurred_at,
        request_id,
    }
}

async fn active_others(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    call_id: Uuid,
    exclude: Uuid,
) -> Result<Vec<Uuid>, RepositoryError> {
    let rows = sqlx::query!(
        r#"
        SELECT user_id
        FROM call_participants
        WHERE call_id = $1
          AND user_id <> $2
          AND status IN ('invited', 'joined')
        "#,
        call_id,
        exclude,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    Ok(rows.into_iter().map(|r| r.user_id).collect())
}

async fn end_call_if_empty(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    call_id: Uuid,
    actor: Uuid,
    request_id: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), RepositoryError> {
    let joined = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM call_participants WHERE call_id = $1 AND status = 'joined'"#,
        call_id,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    if joined > 0 {
        return Ok(());
    }

    let ringing: Vec<Uuid> = sqlx::query!(
        r#"SELECT user_id FROM call_participants WHERE call_id = $1 AND status = 'invited'"#,
        call_id,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?
    .into_iter()
    .map(|r| r.user_id)
    .collect();

    if ringing.is_empty() {
        return Ok(());
    }

    sqlx::query!(
        r#"
        UPDATE call_participants
        SET status = 'left', updated_at = $2
        WHERE call_id = $1 AND status = 'invited'
        "#,
        call_id,
        now,
    )
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    let record = call_event(call_id, "call.ended", actor, &ringing, request_id, now);
    save_outbox_records(tx, std::slice::from_ref(&record)).await?;
    Ok(())
}

#[async_trait]
impl CallRepository for PostgresCallRepository {
    async fn start_call(
        &self,
        call_id: Uuid,
        caller: &UserId,
        invitees: &[UserId],
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
            INSERT INTO call_participants (call_id, user_id, inviter_id, status, created_at, updated_at)
            VALUES ($1, $2, $2, 'joined', $3, $3)
            ON CONFLICT (call_id, user_id) DO NOTHING
            "#,
            call_id,
            caller.uuid(),
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let invitee_ids: Vec<Uuid> = invitees.iter().map(|u| u.uuid()).collect();
        for invitee in &invitee_ids {
            sqlx::query!(
                r#"
                INSERT INTO call_participants (call_id, user_id, inviter_id, status, created_at, updated_at)
                VALUES ($1, $2, $3, 'invited', $4, $4)
                ON CONFLICT (call_id, user_id) DO NOTHING
                "#,
                call_id,
                invitee,
                caller.uuid(),
                now,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }

        let record = call_event(call_id, "call.ring", caller.uuid(), &invitee_ids, request_id, now);
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn respond(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        accept: bool,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let status = if accept {
            CallParticipantStatus::Joined
        } else {
            CallParticipantStatus::Declined
        };
        let updated = sqlx::query!(
            r#"
            UPDATE call_participants
            SET status = $4, updated_at = $3
            WHERE call_id = $1 AND user_id = $2 AND status = 'invited'
            "#,
            call_id,
            user_id.uuid(),
            now,
            status.as_str(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if updated.rows_affected() == 0 {
            return Ok(false);
        }

        if !accept {
            let recipients = active_others(&mut tx, call_id, user_id.uuid()).await?;
            let record =
                call_event(call_id, "call.declined", user_id.uuid(), &recipients, request_id, now);
            save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn add_participants(
        &self,
        call_id: Uuid,
        inviter: &UserId,
        invitees: &[UserId],
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let mut ringed: Vec<Uuid> = Vec::new();
        for invitee in invitees {
            let inserted = sqlx::query!(
                r#"
                INSERT INTO call_participants (call_id, user_id, inviter_id, status, created_at, updated_at)
                VALUES ($1, $2, $3, 'invited', $4, $4)
                ON CONFLICT (call_id, user_id) DO UPDATE
                    SET status = 'invited', session_id = NULL, inviter_id = EXCLUDED.inviter_id, updated_at = EXCLUDED.updated_at
                    WHERE call_participants.status IN ('declined', 'left')
                "#,
                call_id,
                invitee.uuid(),
                inviter.uuid(),
                now,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            if inserted.rows_affected() > 0 {
                ringed.push(invitee.uuid());
            }
        }

        if !ringed.is_empty() {
            let record = call_event(call_id, "call.ring", inviter.uuid(), &ringed, request_id, now);
            save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn leave(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let updated = sqlx::query!(
            r#"
            UPDATE call_participants
            SET status = 'left', updated_at = $3
            WHERE call_id = $1 AND user_id = $2 AND status IN ('invited', 'joined')
            "#,
            call_id,
            user_id.uuid(),
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if updated.rows_affected() == 0 {
            return Ok(());
        }

        let recipients = active_others(&mut tx, call_id, user_id.uuid()).await?;
        let record = call_event(
            call_id,
            "call.participant.left",
            user_id.uuid(),
            &recipients,
            request_id.clone(),
            now,
        );
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        end_call_if_empty(&mut tx, call_id, user_id.uuid(), request_id, now).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn is_active_participant(
        &self,
        call_id: Uuid,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM call_participants
                WHERE call_id = $1 AND user_id = $2 AND status IN ('invited', 'joined')
            ) AS "exists!"
            "#,
            call_id,
            user_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.exists)
    }

    async fn is_joined_participant(
        &self,
        call_id: Uuid,
        user_id: &UserId,
    ) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM call_participants
                WHERE call_id = $1 AND user_id = $2 AND status = 'joined'
            ) AS "exists!"
            "#,
            call_id,
            user_id.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.exists)
    }

    async fn list_active(
        &self,
        call_id: Uuid,
    ) -> Result<Vec<domain::call::repository::CallParticipantView>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT p.user_id, u.name AS user_name
            FROM call_participants p
            INNER JOIN users u ON u.id = p.user_id
            WHERE p.call_id = $1 AND p.status = 'joined'
            "#,
            call_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| domain::call::repository::CallParticipantView {
                user_id: UserId::new(r.user_id),
                user_name: r.user_name,
            })
            .collect())
    }

    async fn active_call_for(&self, user_id: &UserId) -> Result<Option<Uuid>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT call_id
            FROM call_participants
            WHERE user_id = $1 AND status = 'joined'
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
            user_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| r.call_id))
    }

    async fn is_call_room(&self, call_id: Uuid) -> Result<bool, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM call_participants WHERE call_id = $1
            ) AS "exists!"
            "#,
            call_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.exists)
    }

    async fn webhook_joined(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        session_id: &str,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let updated = sqlx::query!(
            r#"
            UPDATE call_participants
            SET status = 'joined', session_id = $3, updated_at = $4
            WHERE call_id = $1 AND user_id = $2
              AND (
                    (status = 'joined' AND session_id IS NULL)
                 OR (status IN ('invited', 'joined') AND $4 >= updated_at)
                 OR (status = 'left' AND $4 > updated_at)
              )
            "#,
            call_id,
            user_id.uuid(),
            session_id,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if updated.rows_affected() == 0 {
            let is_call = sqlx::query!(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM call_participants WHERE call_id = $1
                ) AS "exists!"
                "#,
                call_id,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            return Ok(is_call.exists);
        }

        let recipients = active_others(&mut tx, call_id, user_id.uuid()).await?;
        let record = call_event(
            call_id,
            "call.participant.joined",
            user_id.uuid(),
            &recipients,
            request_id,
            now,
        );
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn webhook_left(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        session_id: &str,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let updated = sqlx::query!(
            r#"
            UPDATE call_participants
            SET status = 'left', updated_at = $4
            WHERE call_id = $1 AND user_id = $2
              AND status IN ('invited', 'joined')
              AND (session_id = $3 OR (session_id IS NULL AND $4 >= updated_at))
            "#,
            call_id,
            user_id.uuid(),
            session_id,
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if updated.rows_affected() == 0 {
            let is_call = sqlx::query!(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM call_participants WHERE call_id = $1
                ) AS "exists!"
                "#,
                call_id,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            return Ok(is_call.exists);
        }

        let recipients = active_others(&mut tx, call_id, user_id.uuid()).await?;
        let record = call_event(
            call_id,
            "call.participant.left",
            user_id.uuid(),
            &recipients,
            request_id.clone(),
            now,
        );
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        end_call_if_empty(&mut tx, call_id, user_id.uuid(), request_id, now).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }
}
