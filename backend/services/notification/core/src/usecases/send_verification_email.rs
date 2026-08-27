use std::sync::Arc;

use domain::user::{email::Email, locale::Locale};
use ports::mailer::{Mailer, MailerError};

#[derive(Debug, Clone)]
pub struct VerificationIssued {
    pub email: Email,
    pub code: String,
    pub locale: Locale,
}

pub struct SendVerificationEmail<M: Mailer> {
    mailer: Arc<M>,
}

impl<M: Mailer> SendVerificationEmail<M> {
    pub fn new(mailer: Arc<M>) -> Self {
        Self { mailer }
    }

    pub async fn execute(&self, issued: VerificationIssued) -> Result<(), MailerError> {
        self.mailer
            .send_verification_code(&issued.email, &issued.code, issued.locale)
            .await
    }
}
