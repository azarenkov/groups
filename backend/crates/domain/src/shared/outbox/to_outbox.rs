use crate::shared::outbox::outbox_record::OutboxRecord;

#[derive(Debug, thiserror::Error)]
pub enum OutboxError {
    #[error("failed to serialize event payload: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub trait ToOutboxRecord {
    fn to_outbox_record(&self) -> Result<OutboxRecord, OutboxError>;

    fn to_outbox_record_with_request_id(
        &self,
        request_id: Option<String>,
    ) -> Result<OutboxRecord, OutboxError> {
        let mut record = self.to_outbox_record()?;
        record.request_id = request_id;
        Ok(record)
    }
}
