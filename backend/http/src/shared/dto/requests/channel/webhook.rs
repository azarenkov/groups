use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveKitWebhookRequestBody {
    pub event: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}
