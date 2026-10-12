use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope<'a> {
    pub r#type: &'a str,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<i64>,
}

impl<'a> Envelope<'a> {
    pub fn new(r#type: &'a str, data: Value) -> Self {
        Self { r#type, data, partition: None, seq: None }
    }

    pub fn sequenced(&self, partition: &str, seq: i64) -> Envelope<'a> {
        Envelope {
            r#type: self.r#type,
            data: self.data.clone(),
            partition: Some(partition.to_string()),
            seq: Some(seq),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_plain_envelope_has_no_sequence_fields() {
        let value = serde_json::to_value(Envelope::new("x", json!({"a": 1}))).unwrap();
        assert_eq!(value, json!({"type": "x", "data": {"a": 1}}));
    }

    #[test]
    fn a_sequenced_envelope_carries_partition_and_seq() {
        let value = serde_json::to_value(Envelope::new("x", json!({})).sequenced("p", 7)).unwrap();
        assert_eq!(value, json!({"type": "x", "data": {}, "partition": "p", "seq": 7}));
    }
}
