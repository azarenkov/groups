use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveKitWebhookRequest {
    pub event: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}
