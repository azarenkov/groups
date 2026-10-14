use crate::messaging::partitions::{outbox_partition_key, partition_for, partition_token};
use ports::{outbox::record::OutboxRecord, outbox::record::PartitionSequence, outbox_publisher::OutboxPublisher, outbox_publisher::OutboxPublisherError};
use async_nats::{HeaderMap, jetstream::Context};
use async_trait::async_trait;

pub const MSG_ID_HEADER: &str = "Nats-Msg-Id";
const REQUEST_ID_HEADER: &str = "X-Request-Id";
pub const PARTITION_HEADER: &str = "Rift-Partition";
pub const PARTITION_SEQ_HEADER: &str = "Rift-Partition-Seq";
pub const PARTITION_MEMBERS_HEADER: &str = "Rift-Partition-Members";

pub struct NatsOutboxPublisher {
    jetstream: Context,
    subject_prefix: String,
    partitions: u32,
}

impl NatsOutboxPublisher {
    pub fn new(jetstream: Context, subject_prefix: String, partitions: u32) -> Self {
        Self {
            jetstream,
            subject_prefix,
            partitions,
        }
    }

    fn subject_for(&self, record: &OutboxRecord) -> String {
        partitioned_subject(&self.subject_prefix, record, self.partitions)
    }
}

pub fn partitioned_subject(prefix: &str, record: &OutboxRecord, partitions: u32) -> String {
    let key = outbox_partition_key(record);
    let partition = partition_for(key, partitions);
    format!(
        "{}.{}.{}.{}",
        prefix,
        record.aggregate_type.as_str(),
        partition_token(partition),
        record.event_type,
    )
}

#[async_trait]
impl OutboxPublisher for NatsOutboxPublisher {
    async fn publish(
        &self,
        record: &OutboxRecord,
        sequence: Option<PartitionSequence>,
    ) -> Result<(), OutboxPublisherError> {
        let subject = self.subject_for(record);
        let payload = serde_json::to_vec(&record.payload)
            .map_err(|e| OutboxPublisherError::Serialization(e.to_string()))?;

        let mut headers = HeaderMap::new();
        headers.insert(MSG_ID_HEADER, record.id.to_string().as_str());
        if let Some(request_id) = record.request_id.as_deref() {
            headers.insert(REQUEST_ID_HEADER, request_id);
        }
        if let Some(sequence) = sequence {
            headers.insert(PARTITION_HEADER, sequence.partition.as_str());
            headers.insert(PARTITION_SEQ_HEADER, sequence.seq.to_string().as_str());
            if !sequence.members.is_empty() {
                let members: Vec<String> = sequence.members.iter().map(|m| m.to_string()).collect();
                headers.insert(PARTITION_MEMBERS_HEADER, members.join(",").as_str());
            }
        }

        let ack_future = self
            .jetstream
            .publish_with_headers(subject, headers, payload.into())
            .await
            .map_err(|e| OutboxPublisherError::Publish(e.to_string()))?;

        ack_future
            .await
            .map_err(|e| OutboxPublisherError::Ack(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ports::outbox::record::{AggregateType, OutboxRecord};
    use uuid::Uuid;

    fn record(parent_id: Option<Uuid>) -> OutboxRecord {
        OutboxRecord {
            id: Uuid::from_u128(1),
            aggregate_id: Uuid::from_u128(2),
            aggregate_type: AggregateType::Message,
            parent_id,
            parent_type: None,
            event_type: "sent".to_string(),
            payload: serde_json::json!({}),
            occurred_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
            request_id: None,
        }
    }

    #[test]
    fn subject_carries_partition_from_parent_key() {
        let parent = Uuid::from_u128(42);
        let a = subject_of(record(Some(parent)));
        let b = subject_of(record(Some(parent)));
        assert_eq!(a, b);
        let parts: Vec<&str> = a.split('.').collect();
        assert_eq!(parts[0..2], ["rift", "events"]);
        assert_eq!(parts[2], "message");
        assert!(crate::messaging::partitions::is_partition_token(parts[3]));
        assert_eq!(parts[4], "sent");
        assert_eq!(
            crate::messaging::partitions::normalize_subject(&a),
            "rift.events.message.sent",
        );
    }

    #[test]
    fn dm_messages_partition_by_conversation_pair() {
        let a = Uuid::from_u128(11);
        let b = Uuid::from_u128(22);
        let mut forward = record(Some(b));
        forward.aggregate_type = AggregateType::DmMessage;
        forward.payload = serde_json::json!({
            "sender_id": a.to_string(),
            "recipient_id": b.to_string(),
        });
        let mut backward = record(Some(a));
        backward.aggregate_type = AggregateType::DmMessage;
        backward.payload = serde_json::json!({
            "sender_id": b.to_string(),
            "recipient_id": a.to_string(),
        });
        assert_eq!(subject_of(forward), subject_of(backward));
    }

    #[test]
    fn friendship_events_partition_by_pair() {
        let a = Uuid::from_u128(7);
        let b = Uuid::from_u128(9);
        let mut sent = record(Some(b));
        sent.aggregate_type = AggregateType::User;
        sent.aggregate_id = a;
        sent.event_type = "request_sent".to_string();
        let mut accepted = record(Some(a));
        accepted.aggregate_type = AggregateType::User;
        accepted.aggregate_id = b;
        accepted.event_type = "request_sent".to_string();
        assert_eq!(subject_of(sent), subject_of(accepted));
    }

    #[test]
    fn dm_without_pair_falls_back_to_parent() {
        let mut rec = record(Some(Uuid::from_u128(5)));
        rec.aggregate_type = AggregateType::DmMessage;
        let subject = subject_of(rec);
        assert!(subject.starts_with("rift.events.dm_message."));
    }

    #[test]
    fn different_keys_can_land_in_different_partitions() {
        let mut subjects = std::collections::HashSet::new();
        for i in 0..32u128 {
            subjects.insert(subject_of(record(Some(Uuid::from_u128(i)))));
        }
        assert!(subjects.len() > 4);
    }

    fn subject_of(record: OutboxRecord) -> String {
        super::partitioned_subject("rift.events", &record, 16)
    }
}
