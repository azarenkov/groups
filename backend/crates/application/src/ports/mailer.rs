use async_trait::async_trait;
use domain::user::email::Email;

#[async_trait]
pub trait Mailer {
    async fn send_password_reset(&self, email: &Email, token: &str) -> Result<(), MailerError>;
    async fn send_verification_code(
        &self,
        email: &Email,
        verification_code: &str,
    ) -> Result<(), MailerError>;
}

#[derive(Debug, thiserror::Error)]
pub enum MailerError {
    #[error("Mailer error: {0}")]
    Sending(String),
}
