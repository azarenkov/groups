use async_trait::async_trait;
use chrono::{DateTime, Utc};
use contracts::events::call::CallInvitationV1;
use voice_domain::{call::repository::AddParticipantsOutcome, call::repository::CallPairOutcome, call::repository::CallRepository, call::repository::JoinOutcome, call::repository::PendingEviction, call::repository::WebhookJoinOutcome, call::CallParticipantStatus, call::CallRoster};
use domain::user::id::UserId;
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::outbox::mapping::encode;
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

async fn joinable_others(
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
          AND status IN ('invited', 'declined', 'left')
        "#,
        call_id,
        exclude,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    Ok(rows.into_iter().map(|r| r.user_id).collect())
}

async fn mark_left(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    call_id: Uuid,
    user_id: Uuid,
    now: DateTime<Utc>,
    request_id: Option<String>,
) -> Result<bool, RepositoryError> {
    let previous = participant_status(tx, call_id, user_id).await?;
    let updated = sqlx::query!(
        r#"
            UPDATE call_participants
            SET status = 'left', updated_at = $3
            WHERE call_id = $1 AND user_id = $2 AND status IN ('invited', 'joined')
            "#,
        call_id,
        user_id,
        now,
    )
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    if updated.rows_affected() == 0 {
        return Ok(false);
    }
    let recipients = active_others(tx, call_id, user_id).await?;
    let record = call_event(
        call_id,
        "call.participant.left",
        user_id,
        &recipients,
        request_id.clone(),
        now,
    );
    save_outbox_records(tx, std::slice::from_ref(&record)).await?;
    notify_withdrawn_invitation(tx, call_id, user_id, previous.as_deref(), request_id, now).await?;
    Ok(true)
}

async fn cancel_eviction(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    call_id: Uuid,
    user_id: Uuid,
) -> Result<(), RepositoryError> {
    sqlx::query("DELETE FROM call_evictions WHERE call_id = $1 AND user_id = $2")
        .bind(call_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
    Ok(())
}

async fn participant_status(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    call_id: Uuid,
    user_id: Uuid,
) -> Result<Option<String>, RepositoryError> {
    sqlx::query_scalar::<_, String>(
        "SELECT status FROM call_participants WHERE call_id = $1 AND user_id = $2 FOR UPDATE",
    )
    .bind(call_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)
}

async fn notify_withdrawn_invitation(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    call_id: Uuid,
    user_id: Uuid,
    previous_status: Option<&str>,
    request_id: Option<String>,
    now: DateTime<Utc>,
) -> Result<(), RepositoryError> {
    if previous_status != Some("invited") {
        return Ok(());
    }
    let record = call_event(call_id, "call.ended", user_id, &[user_id], request_id, now);
    save_outbox_records(tx, std::slice::from_ref(&record)).await
}

async fn end_call_when(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    call_id: Uuid,
    actor: Uuid,
    request_id: Option<String>,
    now: DateTime<Utc>,
    is_over: fn(&CallRoster) -> bool,
) -> Result<(), RepositoryError> {
    let roster = sqlx::query!(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status = 'invited') AS "ringing!",
            COUNT(*) FILTER (WHERE status = 'joined' AND (session_id IS NOT NULL OR updated_at > $2)) AS "connected!"
        FROM call_participants
        WHERE call_id = $1
        "#,
        call_id,
        now - chrono::Duration::seconds(60),
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;
    let roster = CallRoster { ringing: roster.ringing as usize, connected: roster.connected as usize };
    if !is_over(&roster) {
        return Ok(());
    }

    let participants = sqlx::query!(
        r#"SELECT user_id, inviter_id FROM call_participants WHERE call_id = $1"#,
        call_id,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    sqlx::query!(
        r#"
        UPDATE call_participants
        SET status = 'left', updated_at = $2
        WHERE call_id = $1 AND status IN ('invited', 'joined')
        "#,
        call_id,
        now,
    )
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_error_to_domain_error)?;

    if participants.is_empty() {
        return Ok(());
    }
    let recipients: Vec<Uuid> = participants.iter().map(|p| p.user_id).collect();
    let invitations: Vec<CallInvitationV1> = participants
        .iter()
        .filter(|p| p.inviter_id != p.user_id)
        .map(|p| CallInvitationV1 { inviter_id: p.inviter_id, invitee_id: p.user_id })
        .collect();
    let mut record = call_event(call_id, "call.ended", actor, &recipients, request_id, now);
    record.payload["invitations"] = encode(&invitations)?;
    save_outbox_records(tx, std::slice::from_ref(&record)).await
}

fn call_ring_event(
    call_id: Uuid,
    actor_id: Uuid,
    recipients: &[Uuid],
    video: bool,
    request_id: Option<String>,
    occurred_at: DateTime<Utc>,
) -> OutboxRecord {
    let mut record = call_event(call_id, "call.ring", actor_id, recipients, request_id, occurred_at);
    record.payload["video"] = serde_json::Value::Bool(video);
    record
}

async fn stored_call_video<'e, E>(executor: E, call_id: Uuid) -> Result<bool, RepositoryError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let video: Option<bool> = sqlx::query_scalar("SELECT video FROM calls WHERE call_id = $1")
        .bind(call_id)
        .fetch_optional(executor)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
    Ok(video.unwrap_or(false))
}

#[async_trait]
impl CallRepository for PostgresCallRepository {
    async fn start_call(
        &self,
        call_id: Uuid,
        caller: &UserId,
        invitees: &[UserId],
        video: bool,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query(
            "INSERT INTO calls (call_id, video, created_at) VALUES ($1, $2, $3) ON CONFLICT (call_id) DO NOTHING",
        )
        .bind(call_id)
        .bind(video)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
            INSERT INTO call_participants (call_id, user_id, inviter_id, status, created_at, updated_at, joined_at)
            VALUES ($1, $2, $2, 'joined', $3, $3, $3)
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

        let record = call_ring_event(call_id, caller.uuid(), &invitee_ids, video, request_id, now);
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn call_video(&self, call_id: Uuid) -> Result<bool, RepositoryError> {
        stored_call_video(&self.pool, call_id).await
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
            SET status = $4, updated_at = $3, joined_at = CASE WHEN $4::text = 'joined' THEN $3::timestamptz ELSE NULL END
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
                call_event(call_id, "call.declined", user_id.uuid(), &recipients, request_id.clone(), now);
            save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;
            end_call_when(&mut tx, call_id, user_id.uuid(), request_id, now, CallRoster::is_over_after_decline).await?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }

    async fn add_participants(
        &self,
        call_id: Uuid,
        inviter: &UserId,
        invitees: &[UserId],
        max_participants: usize,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<AddParticipantsOutcome, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 44))")
            .bind(call_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let existing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM call_participants WHERE call_id = $1 AND user_id <> $2 AND status IN ('invited', 'joined')",
        )
        .bind(call_id)
        .bind(inviter.uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let requested: Vec<Uuid> = invitees.iter().map(|u| u.uuid()).collect();
        let already_active: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT user_id FROM call_participants WHERE call_id = $1 AND user_id = ANY($2) AND status IN ('invited', 'joined')",
        )
        .bind(call_id)
        .bind(&requested)
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let fresh = requested
            .iter()
            .filter(|id| !already_active.iter().any(|(active,)| active == *id))
            .count();
        if usize::try_from(existing).unwrap_or(usize::MAX).saturating_add(fresh) > max_participants {
            return Ok(AddParticipantsOutcome::CapExceeded);
        }

        let mut ringed: Vec<Uuid> = Vec::new();
        for invitee in invitees {
            sqlx::query!(
                r#"
                INSERT INTO call_invite_outcomes (call_id, user_id, inviter_id, answered_at, declined, left_at, created_at)
                SELECT call_id, user_id, inviter_id, answered_at, status = 'declined',
                       CASE WHEN status = 'left' THEN updated_at END, $3
                FROM call_participants
                WHERE call_id = $1 AND user_id = $2 AND status IN ('declined', 'left')
                "#,
                call_id,
                invitee.uuid(),
                now,
            )
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            let inserted = sqlx::query!(
                r#"
                INSERT INTO call_participants (call_id, user_id, inviter_id, status, created_at, updated_at)
                VALUES ($1, $2, $3, 'invited', $4, $4)
                ON CONFLICT (call_id, user_id) DO UPDATE
                    SET status = 'invited', session_id = NULL, answered_at = NULL, joined_at = NULL, inviter_id = EXCLUDED.inviter_id, updated_at = EXCLUDED.updated_at
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
                cancel_eviction(&mut tx, call_id, invitee.uuid()).await?;
                ringed.push(invitee.uuid());
            }
        }

        if !ringed.is_empty() {
            let video = stored_call_video(&mut *tx, call_id).await?;
            let record = call_ring_event(call_id, inviter.uuid(), &ringed, video, request_id, now);
            save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(AddParticipantsOutcome::Added(ringed.into_iter().map(UserId::new).collect()))
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

        let previous = participant_status(&mut tx, call_id, user_id.uuid()).await?;
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
        notify_withdrawn_invitation(&mut tx, call_id, user_id.uuid(), previous.as_deref(), request_id.clone(), now).await?;

        end_call_when(&mut tx, call_id, user_id.uuid(), request_id, now, CallRoster::is_abandoned).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn leave_many(
        &self,
        call_id: Uuid,
        user_ids: &[UserId],
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError> {
        let Some(actor) = user_ids.last() else {
            return Ok(());
        };
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        for user_id in user_ids {
            mark_left(&mut tx, call_id, user_id.uuid(), now, request_id.clone()).await?;
        }

        end_call_when(&mut tx, call_id, actor.uuid(), request_id, now, CallRoster::is_abandoned).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn evict_many(
        &self,
        call_id: Uuid,
        user_ids: &[UserId],
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<(), RepositoryError> {
        let Some(actor) = user_ids.last() else {
            return Ok(());
        };
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        for user_id in user_ids {
            mark_left(&mut tx, call_id, user_id.uuid(), now, request_id.clone()).await?;
            sqlx::query(
                "INSERT INTO call_evictions (call_id, user_id, next_attempt_at) VALUES ($1, $2, $3) ON CONFLICT (call_id, user_id) DO UPDATE SET next_attempt_at = EXCLUDED.next_attempt_at",
            )
            .bind(call_id)
            .bind(user_id.uuid())
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }

        end_call_when(&mut tx, call_id, actor.uuid(), request_id, now, CallRoster::is_abandoned).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn claim_due_evictions(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<PendingEviction>, RepositoryError> {
        sqlx::query(
            r#"
            DELETE FROM call_evictions e
            WHERE EXISTS (
                SELECT 1 FROM call_participants p
                WHERE p.call_id = e.call_id AND p.user_id = e.user_id AND p.status IN ('invited', 'joined')
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let rows: Vec<(Uuid, Uuid, i32)> = sqlx::query_as(
            r#"
            UPDATE call_evictions e
            SET attempts = e.attempts + 1, next_attempt_at = $1 + interval '30 seconds'
            WHERE (e.call_id, e.user_id) IN (
                SELECT c.call_id, c.user_id FROM call_evictions c
                WHERE c.next_attempt_at <= $1
                  AND NOT EXISTS (
                      SELECT 1 FROM call_participants p
                      WHERE p.call_id = c.call_id AND p.user_id = c.user_id AND p.status IN ('invited', 'joined')
                  )
                ORDER BY c.next_attempt_at
                LIMIT $2
                FOR UPDATE SKIP LOCKED
            )
            RETURNING e.call_id, e.user_id, e.attempts
            "#,
        )
        .bind(now)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|(call_id, user_id, attempts)| PendingEviction {
                call_id,
                user_id: UserId::new(user_id),
                attempts,
            })
            .collect())
    }

    async fn eviction_done(&self, call_id: Uuid, user_id: &UserId) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM call_evictions WHERE call_id = $1 AND user_id = $2")
            .bind(call_id)
            .bind(user_id.uuid())
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn eviction_failed(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        next_attempt_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query("UPDATE call_evictions SET next_attempt_at = $3 WHERE call_id = $1 AND user_id = $2")
            .bind(call_id)
            .bind(user_id.uuid())
            .bind(next_attempt_at)
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
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
    ) -> Result<Vec<voice_domain::call::repository::CallParticipantView>, RepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT p.user_id, u.name AS user_name
            FROM call_participants p
            INNER JOIN voice_user_profiles u ON u.user_id = p.user_id
            WHERE p.call_id = $1 AND p.status = 'joined'
            "#,
            call_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| voice_domain::call::repository::CallParticipantView {
                user_id: UserId::new(r.user_id),
                user_name: r.user_name,
            })
            .collect())
    }

    async fn active_participant_ids(&self, call_id: Uuid) -> Result<Vec<UserId>, RepositoryError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT user_id FROM call_participants WHERE call_id = $1 AND status IN ('invited', 'joined')",
        )
        .bind(call_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows.into_iter().map(|(id,)| UserId::new(id)).collect())
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

    async fn joined_at(
        &self,
        call_id: Uuid,
        user_id: &UserId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        let row: Option<(Option<DateTime<Utc>>,)> = sqlx::query_as(
            "SELECT COALESCE(joined_at, answered_at, updated_at) FROM call_participants WHERE call_id = $1 AND user_id = $2 AND status = 'joined'",
        )
        .bind(call_id)
        .bind(user_id.uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.and_then(|(at,)| at))
    }

    async fn shared_active_calls(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Vec<Uuid>, RepositoryError> {
        sqlx::query_scalar(
            r#"
            SELECT p1.call_id
            FROM call_participants p1
            INNER JOIN call_participants p2
                ON p2.call_id = p1.call_id
                AND p2.user_id = $2
                AND p2.status IN ('invited', 'joined')
            WHERE p1.user_id = $1 AND p1.status IN ('invited', 'joined')
            "#,
        )
        .bind(user_a.uuid())
        .bind(user_b.uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn active_call_between(
        &self,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<Option<Uuid>, RepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT p1.call_id
            FROM call_participants p1
            INNER JOIN call_participants p2
                ON p2.call_id = p1.call_id
                AND p2.user_id = $2
                AND p2.status = 'joined'
            WHERE p1.user_id = $1
            ORDER BY p1.updated_at DESC
            LIMIT 1
            "#,
            user_a.uuid(),
            user_b.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(row.map(|r| r.call_id))
    }

    async fn join(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        max_participants: usize,
        now: DateTime<Utc>,
    ) -> Result<JoinOutcome, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 44))")
            .bind(call_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        let current = participant_status(&mut tx, call_id, user_id.uuid()).await?;
        let reactivating = !matches!(current.as_deref(), Some("invited") | Some("joined"));
        if reactivating {
            let active: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM call_participants WHERE call_id = $1 AND user_id <> $2 AND status IN ('invited', 'joined')",
            )
            .bind(call_id)
            .bind(user_id.uuid())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            if usize::try_from(active).unwrap_or(usize::MAX) > max_participants {
                return Ok(JoinOutcome::CapExceeded);
            }
        }
        let updated = sqlx::query!(
            r#"
            UPDATE call_participants
            SET status = 'joined', session_id = NULL, updated_at = $3,
                joined_at = CASE WHEN status = 'joined' THEN COALESCE(joined_at, $3) ELSE $3 END
            WHERE call_id = $1 AND user_id = $2
              AND status IN ('invited', 'declined', 'left', 'joined')
              AND (
                status = 'joined'
                OR EXISTS(
                    SELECT 1 FROM call_participants o
                    WHERE o.call_id = $1
                      AND o.user_id <> $2
                      AND o.status = 'joined'
                )
              )
            "#,
            call_id,
            user_id.uuid(),
            now,
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if updated.rows_affected() == 0 {
            return Ok(JoinOutcome::NotParticipant);
        }
        cancel_eviction(&mut tx, call_id, user_id.uuid()).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(JoinOutcome::Joined)
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

    async fn pair_outcome(
        &self,
        call_id: Uuid,
        caller: &UserId,
        callee: &UserId,
    ) -> Result<CallPairOutcome, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT user_id, inviter_id, status, answered_at, updated_at FROM call_participants WHERE call_id = $1 AND user_id IN ($2, $3)"#,
            call_id,
            caller.uuid(),
            callee.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let caller_present = rows.iter().any(|r| r.user_id == caller.uuid());
        let callee_row = rows.iter().find(|r| r.user_id == callee.uuid());
        let archived = sqlx::query!(
            r#"
            SELECT
                COUNT(*) > 0 AS "present!",
                MIN(answered_at) AS earliest_answer,
                BOOL_OR(declined) AS declined
            FROM call_invite_outcomes
            WHERE call_id = $1 AND user_id = $2 AND inviter_id = $3
            "#,
            call_id,
            callee.uuid(),
            caller.uuid(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let current = callee_row.filter(|r| r.inviter_id == caller.uuid());
        let callee_invited_by_caller = current.is_some() || archived.present;
        let callee_answered_at = match (current.and_then(|r| r.answered_at), archived.earliest_answer)
        {
            (Some(c), Some(a)) => Some(c.min(a)),
            (c, a) => c.or(a),
        };
        let callee_declined = callee_answered_at.is_none()
            && (current.map(|r| r.status == "declined").unwrap_or(false)
                || archived.declined.unwrap_or(false));
        let callee_pending = current
            .map(|r| {
                r.status == "invited"
                    || (r.status == "joined"
                        && r.answered_at.is_none()
                        && r.updated_at > Utc::now() - chrono::Duration::seconds(60))
            })
            .unwrap_or(false);
        let caller_archive = sqlx::query!(
            r#"
            SELECT MIN(left_at) AS earliest_left
            FROM call_invite_outcomes
            WHERE call_id = $1 AND user_id = $2
              AND ($3::timestamptz IS NULL OR left_at >= $3)
            "#,
            call_id,
            caller.uuid(),
            callee_answered_at,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let current_caller_left = rows
            .iter()
            .find(|r| r.user_id == caller.uuid() && r.status == "left")
            .map(|r| r.updated_at)
            .filter(|ts| callee_answered_at.map(|a| *ts >= a).unwrap_or(true));
        let caller_left_at = match (current_caller_left, caller_archive.earliest_left) {
            (Some(c), Some(a)) => Some(c.min(a)),
            (c, a) => c.or(a),
        };
        let caller_in_call = rows
            .iter()
            .any(|r| r.user_id == caller.uuid() && r.status == "joined")
            && !(callee_answered_at.is_some() && caller_left_at.is_some());
        Ok(CallPairOutcome {
            participated: caller_present && callee_row.is_some(),
            caller_in_call,
            caller_left_at,
            callee_invited_by_caller,
            callee_pending,
            callee_answered_at,
            callee_declined,
        })
    }

    async fn webhook_joined(
        &self,
        call_id: Uuid,
        user_id: &UserId,
        session_id: &str,
        max_participants: usize,
        now: DateTime<Utc>,
        request_id: Option<String>,
    ) -> Result<WebhookJoinOutcome, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 44))")
            .bind(call_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let current = participant_status(&mut tx, call_id, user_id.uuid()).await?;
        if !matches!(current.as_deref(), Some("invited") | Some("joined")) {
            let eviction_pending: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM call_evictions WHERE call_id = $1 AND user_id = $2)",
            )
            .bind(call_id)
            .bind(user_id.uuid())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            if eviction_pending {
                return Ok(WebhookJoinOutcome::EvictionPending);
            }
            let active: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM call_participants WHERE call_id = $1 AND user_id <> $2 AND status IN ('invited', 'joined')",
            )
            .bind(call_id)
            .bind(user_id.uuid())
            .fetch_one(&mut *tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            if current.is_some() && usize::try_from(active).unwrap_or(usize::MAX) > max_participants {
                return Ok(WebhookJoinOutcome::CallFull);
            }
        }

        let updated = sqlx::query!(
            r#"
            UPDATE call_participants
            SET status = 'joined', session_id = $3, updated_at = $4, answered_at = COALESCE(answered_at, $4),
                joined_at = CASE WHEN status = 'joined' THEN COALESCE(joined_at, $4) ELSE $4 END
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
            return Ok(if is_call.exists {
                WebhookJoinOutcome::Joined
            } else {
                WebhookJoinOutcome::NotCallRoom
            });
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

        let joinable = joinable_others(&mut tx, call_id, user_id.uuid()).await?;
        if !joinable.is_empty() {
            let avail = call_event(call_id, "call.available", user_id.uuid(), &joinable, None, now);
            save_outbox_records(&mut tx, std::slice::from_ref(&avail)).await?;
        }

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(WebhookJoinOutcome::Joined)
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
              AND status = 'joined'
              AND session_id = $3
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

        end_call_when(&mut tx, call_id, user_id.uuid(), request_id, now, CallRoster::is_abandoned).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(true)
    }
}
