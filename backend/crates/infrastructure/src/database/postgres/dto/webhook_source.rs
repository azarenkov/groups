use application::ports::webhook_inbox::WebhookSource;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "webhook_source", rename_all = "snake_case")]
pub enum WebhookSourceDto {
    LiveKit,
}

impl From<WebhookSource> for WebhookSourceDto {
    fn from(value: WebhookSource) -> Self {
        match value {
            WebhookSource::LiveKit => Self::LiveKit,
        }
    }
}
