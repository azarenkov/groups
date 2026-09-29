use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::user::{email::Email, locale::Locale};
use ports::mailer::{Mailer, MailerError};

#[derive(Debug, Clone)]
pub struct TrialEnding {
    pub email: Email,
    pub ends_at: DateTime<Utc>,
    pub locale: Locale,
}

pub struct SendTrialEndingEmail<M: Mailer> {
    mailer: Arc<M>,
}

impl<M: Mailer> SendTrialEndingEmail<M> {
    pub fn new(mailer: Arc<M>) -> Self {
        Self { mailer }
    }

    pub async fn execute(&self, ending: TrialEnding, now: DateTime<Utc>) -> Result<(), MailerError> {
        if ending.ends_at <= now {
            return Ok(());
        }
        self.mailer
            .send_trial_ending(&ending.email, ending.ends_at, ending.locale)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Duration;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeMailer {
        sent: Mutex<Vec<(String, DateTime<Utc>, Locale)>>,
    }

    #[async_trait]
    impl Mailer for FakeMailer {
        async fn send_password_reset(&self, _: &Email, _: &str, _: Locale) -> Result<(), MailerError> {
            Ok(())
        }

        async fn send_verification_code(&self, _: &Email, _: &str, _: Locale) -> Result<(), MailerError> {
            Ok(())
        }

        async fn send_trial_ending(&self, email: &Email, ends_at: DateTime<Utc>, locale: Locale) -> Result<(), MailerError> {
            self.sent
                .lock()
                .unwrap()
                .push((email.as_str().to_string(), ends_at, locale));
            Ok(())
        }
    }

    fn ending(ends_at: DateTime<Utc>) -> TrialEnding {
        TrialEnding {
            email: Email::new("a@b.c".to_string()),
            ends_at,
            locale: Locale::Ru,
        }
    }

    #[tokio::test]
    async fn mails_a_reminder_for_a_trial_that_is_still_running() {
        let mailer = Arc::new(FakeMailer::default());
        let usecase = SendTrialEndingEmail::new(mailer.clone());
        let now = Utc::now();
        usecase.execute(ending(now + Duration::hours(20)), now).await.unwrap();
        let sent = mailer.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "a@b.c");
        assert_eq!(sent[0].2, Locale::Ru);
    }

    #[tokio::test]
    async fn skips_a_late_reminder_for_a_trial_that_already_ended() {
        let mailer = Arc::new(FakeMailer::default());
        let usecase = SendTrialEndingEmail::new(mailer.clone());
        let now = Utc::now();
        usecase.execute(ending(now - Duration::minutes(1)), now).await.unwrap();
        assert!(mailer.sent.lock().unwrap().is_empty());
    }
}
