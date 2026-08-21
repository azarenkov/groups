use domain::shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError};
use sqlx::{Postgres, Transaction};

use crate::database::postgres::{
    dto::aggregate_type::AggregateTypeDto, error::map_sqlx_error_to_domain_error,
};
pub async fn save_outbox_records(
    tx: &mut Transaction<'_, Postgres>,
    records: &[OutboxRecord],
) -> Result<(), RepositoryError> {
    for record in records {
        let aggregate_type = AggregateTypeDto::from(&record.aggregate_type);
        let parent_type = record.parent_type.as_ref().map(AggregateTypeDto::from);

        sqlx::query!(
            r#"
            INSERT INTO outbox (
                id, aggregate_id, aggregate_type,
                parent_id, parent_type,
                event_type, payload, occurred_at, request_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            record.id,
            record.aggregate_id,
            aggregate_type as AggregateTypeDto,
            record.parent_id,
            parent_type as Option<AggregateTypeDto>,
            record.event_type,
            record.payload,
            record.occurred_at,
            record.request_id,
        )
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
    }
    Ok(())
}
