use ports::outbox::record::{AggregateType, OutboxRecord};
use uuid::Uuid;

pub fn partition_for(key: Uuid, partitions: u32) -> u32 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in key.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash % partitions.max(1) as u64) as u32
}

pub fn partition_token(partition: u32) -> String {
    format!("p{partition}")
}

pub fn is_partition_token(token: &str) -> bool {
    token.len() > 1
        && token.starts_with('p')
        && token[1..].bytes().all(|b| b.is_ascii_digit())
}

pub fn normalize_subject(subject: &str) -> String {
    if !subject.split('.').any(is_partition_token) {
        return subject.to_string();
    }
    subject
        .split('.')
        .filter(|t| !is_partition_token(t))
        .collect::<Vec<_>>()
        .join(".")
}

pub fn outbox_partition_key(record: &OutboxRecord) -> Uuid {
    if matches!(record.aggregate_type, AggregateType::DmMessage)
        && let (Some(a), Some(b)) = (
            payload_uuid(record, "sender_id"),
            payload_uuid(record, "recipient_id"),
        )
    {
        return conversation_key(a, b);
    }
    if matches!(record.aggregate_type, AggregateType::User)
        && let Some(parent) = record.parent_id
    {
        return conversation_key(record.aggregate_id, parent);
    }
    record.parent_id.unwrap_or(record.aggregate_id)
}

fn payload_uuid(record: &OutboxRecord, field: &str) -> Option<Uuid> {
    record
        .payload
        .get(field)
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

pub fn is_conversation_partition(record: &OutboxRecord) -> bool {
    match record.aggregate_type {
        AggregateType::DmMessage | AggregateType::Message => true,
        AggregateType::User => record.parent_id.is_some(),
        _ => false,
    }
}

pub fn partition_id(source: &str, key: Uuid) -> String {
    format!("{source}:{key}")
}

pub fn conversation_key(a: Uuid, b: Uuid) -> Uuid {
    let mut bytes = [0u8; 16];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = a.as_bytes()[i] ^ b.as_bytes()[i];
    }
    Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(aggregate_type: AggregateType, parent_id: Option<Uuid>) -> OutboxRecord {
        OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id: Uuid::new_v4(),
            aggregate_type,
            parent_id,
            parent_type: parent_id.map(|_| AggregateType::User),
            event_type: "e".to_string(),
            payload: serde_json::json!({}),
            occurred_at: chrono::Utc::now(),
            request_id: None,
        }
    }

    #[test]
    fn only_conversation_partitions_are_sequenced() {
        assert!(is_conversation_partition(&record(AggregateType::DmMessage, None)));
        assert!(is_conversation_partition(&record(AggregateType::Message, Some(Uuid::new_v4()))));
        assert!(is_conversation_partition(&record(AggregateType::User, Some(Uuid::new_v4()))));
        assert!(!is_conversation_partition(&record(AggregateType::User, None)));
        assert!(!is_conversation_partition(&record(AggregateType::Storage, None)));
        assert!(!is_conversation_partition(&record(AggregateType::Channel, None)));
        assert!(!is_conversation_partition(&record(AggregateType::Community, None)));
    }

    #[test]
    fn partition_is_deterministic_and_bounded() {
        let key = Uuid::parse_str("0b410a13-a242-46dc-ba1e-cf8692f61af3").unwrap();
        let p1 = partition_for(key, 16);
        let p2 = partition_for(key, 16);
        assert_eq!(p1, p2);
        assert!(p1 < 16);
        assert_eq!(partition_for(key, 1), 0);
    }

    #[test]
    fn partitions_spread_across_buckets() {
        let mut seen = std::collections::HashSet::new();
        for i in 0..64u128 {
            let key = Uuid::from_u128(i * 7919);
            seen.insert(partition_for(key, 16));
        }
        assert!(seen.len() > 8);
    }

    #[test]
    fn normalize_strips_partition_token() {
        assert_eq!(
            normalize_subject("rift.events.user.p3.renamed"),
            "rift.events.user.renamed",
        );
        assert_eq!(
            normalize_subject("rift.events.channel.p15.participant.joined"),
            "rift.events.channel.participant.joined",
        );
        assert_eq!(
            normalize_subject("rift.events.user.renamed"),
            "rift.events.user.renamed",
        );
        assert_eq!(
            normalize_subject("rift.events.dlq.media.community"),
            "rift.events.dlq.media.community",
        );
    }

    #[test]
    fn partition_tokens_are_recognized() {
        assert!(is_partition_token("p0"));
        assert!(is_partition_token("p15"));
        assert!(!is_partition_token("p"));
        assert!(!is_partition_token("participant"));
        assert!(!is_partition_token("user"));
    }
}
