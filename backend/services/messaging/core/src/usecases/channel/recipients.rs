use domain::user::id::UserId;
use ports::outbox::record::OutboxRecord;
use serde_json::{Value, json};

pub fn inject_recipient_ids(records: &mut [OutboxRecord], recipient_ids: &[UserId]) {
    let ids: Vec<Value> = recipient_ids
        .iter()
        .map(|u| Value::String(u.uuid().to_string()))
        .collect();
    for r in records.iter_mut() {
        if let Value::Object(obj) = &mut r.payload {
            obj.insert("recipient_ids".to_string(), json!(ids));
        }
    }
}
