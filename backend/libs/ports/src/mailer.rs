use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{user::email::Email, user::locale::Locale};

#[async_trait]
pub trait Mailer {
    async fn send_password_reset(
        &self,
        email: &Email,
        token: &str,
        locale: Locale,
    ) -> Result<(), MailerError>;
    async fn send_verification_code(
        &self,
        email: &Email,
        verification_code: &str,
        locale: Locale,
    ) -> Result<(), MailerError>;
    async fn send_trial_ending(
        &self,
        email: &Email,
        ends_at: DateTime<Utc>,
        locale: Locale,
    ) -> Result<(), MailerError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MailerError {
    #[error("Mailer error: {0}")]
    Sending(String),
}
