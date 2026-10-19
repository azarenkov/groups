use std::sync::Arc;

use domain::user::{email::Email, locale::Locale};
use ports::mailer::{CodePurpose, Mailer, MailerError};

#[derive(Debug, Clone)]
pub struct VerificationIssued {
    pub email: Email,
    pub code: String,
    pub purpose: String,
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
        if issued.purpose == "password_reset" {
            return self
                .mailer
                .send_password_reset(&issued.email, &issued.code, issued.locale)
                .await;
        }
        let purpose = if issued.purpose == "email_change" {
            CodePurpose::EmailChange
        } else {
            CodePurpose::Registration
        };
        self.mailer
            .send_verification_code(&issued.email, &issued.code, purpose, issued.locale)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    #[async_trait]
    impl Mailer for Recorder {
        async fn send_password_reset(&self, _: &Email, token: &str, _: Locale) -> Result<(), MailerError> {
            self.0.lock().unwrap().push(format!("reset:{token}"));
            Ok(())
        }
        async fn send_verification_code(&self, _: &Email, code: &str, purpose: CodePurpose, _: Locale) -> Result<(), MailerError> {
            self.0.lock().unwrap().push(format!("code:{code}:{purpose:?}"));
            Ok(())
        }
    }

    fn issued(purpose: &str, code: &str) -> VerificationIssued {
        VerificationIssued {
            email: Email::new("t@example.com".into()),
            code: code.into(),
            purpose: purpose.into(),
            locale: Locale::default(),
        }
    }

    #[tokio::test]
    async fn each_purpose_gets_its_own_email() {
        let mailer = Arc::new(Recorder::default());
        let usecase = SendVerificationEmail::new(mailer.clone());
        usecase.execute(issued("password_reset", "tok")).await.unwrap();
        usecase.execute(issued("registration", "123456")).await.unwrap();
        usecase.execute(issued("email_change", "654321")).await.unwrap();
        assert_eq!(*mailer.0.lock().unwrap(), vec!["reset:tok", "code:123456:Registration", "code:654321:EmailChange"]);
    }
}
