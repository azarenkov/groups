use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope<'a> {
    pub r#type: &'a str,
    pub data: Value,
}

impl<'a> Envelope<'a> {
    pub fn new(r#type: &'a str, data: Value) -> Self {
        Self { r#type, data }
    }
}
