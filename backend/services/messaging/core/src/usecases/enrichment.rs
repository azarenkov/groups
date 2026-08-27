use domain::{attachment::Attachment, attachment::AttachmentLink, attachment::AttachmentSnapshot, shared::outbox::outbox_record::OutboxRecord, user::id::UserId};
use serde_json::{Value, json};

pub fn inject_field(records: &mut [OutboxRecord], key: &str, value: Value) {
    for r in records.iter_mut() {
        if let Value::Object(obj) = &mut r.payload {
            obj.insert(key.to_string(), value.clone());
        }
    }
}

pub fn recipient_ids_value(recipient_ids: &[UserId]) -> Value {
    Value::Array(
        recipient_ids
            .iter()
            .map(|u| Value::String(u.uuid().to_string()))
            .collect(),
    )
}

pub fn attachments_value(attachments: &[Attachment]) -> Value {
    json!(
        attachments
            .iter()
            .map(AttachmentSnapshot::from)
            .collect::<Vec<_>>()
    )
}

pub fn attachment_links_value(attachments: &[AttachmentLink]) -> Value {
    json!(
        attachments
            .iter()
            .map(AttachmentSnapshot::from)
            .collect::<Vec<_>>()
    )
}
