use std::sync::Arc;

use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::mailer::{Mailer, MailerError};

use crate::ports::trial_state::TrialState;

#[derive(Debug, Clone)]
pub struct TrialEnding {
    pub user_id: UserId,
    pub ends_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum SendTrialEndingError {
    #[error("Mailer error: {0}")]
    Mailer(#[from] MailerError),
    #[error("Trial state lookup failed: {0}")]
    Lookup(String),
}

pub struct SendTrialEndingEmail<M: Mailer, T: TrialState> {
    mailer: Arc<M>,
    trials: Arc<T>,
}

impl<M: Mailer, T: TrialState> SendTrialEndingEmail<M, T> {
    pub fn new(mailer: Arc<M>, trials: Arc<T>) -> Self {
        Self { mailer, trials }
    }

    pub async fn execute(&self, ending: TrialEnding, now: DateTime<Utc>) -> Result<bool, SendTrialEndingError> {
        if ending.ends_at <= now {
            return Ok(false);
        }
        let Some(current) = self
            .trials
            .active_trial(&ending.user_id)
            .await
            .map_err(SendTrialEndingError::Lookup)?
        else {
            return Ok(false);
        };
        if current.ends_at != ending.ends_at {
            return Ok(false);
        }
        self.mailer
            .send_trial_ending(&current.email, current.ends_at, current.locale)
            .await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::ports::trial_state::ActiveTrial;
    use chrono::Duration;
    use domain::user::{email::Email, locale::Locale};
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

    struct FakeTrials(Result<Option<DateTime<Utc>>, String>);

    #[async_trait]
    impl TrialState for FakeTrials {
        async fn active_trial(&self, _: &UserId) -> Result<Option<ActiveTrial>, String> {
            self.0.clone().map(|end| {
                end.map(|ends_at| ActiveTrial {
                    ends_at,
                    email: Email::new("now@b.c".to_string()),
                    locale: Locale::Ru,
                })
            })
        }
    }

    fn ending(ends_at: DateTime<Utc>) -> TrialEnding {
        TrialEnding {
            user_id: UserId::new(uuid::Uuid::nil()),
            ends_at,
        }
    }

    fn usecase(state: Result<Option<DateTime<Utc>>, String>) -> (Arc<FakeMailer>, SendTrialEndingEmail<FakeMailer, FakeTrials>) {
        let mailer = Arc::new(FakeMailer::default());
        let usecase = SendTrialEndingEmail::new(mailer.clone(), Arc::new(FakeTrials(state)));
        (mailer, usecase)
    }

    #[tokio::test]
    async fn mails_the_current_address_for_the_running_trial() {
        let now = Utc::now();
        let ends_at = now + Duration::hours(20);
        let (mailer, usecase) = usecase(Ok(Some(ends_at)));
        assert!(usecase.execute(ending(ends_at), now).await.unwrap());
        let sent = mailer.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "now@b.c");
        assert_eq!(sent[0].2, Locale::Ru);
    }

    #[tokio::test]
    async fn skips_a_trial_that_already_ended() {
        let now = Utc::now();
        let (mailer, usecase) = usecase(Ok(None));
        assert!(!usecase.execute(ending(now - Duration::minutes(1)), now).await.unwrap());
        assert!(mailer.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn skips_a_reminder_after_the_user_upgraded() {
        let now = Utc::now();
        let (mailer, usecase) = usecase(Ok(None));
        assert!(!usecase.execute(ending(now + Duration::hours(20)), now).await.unwrap());
        assert!(mailer.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn skips_a_reminder_for_an_older_trial_window() {
        let now = Utc::now();
        let (mailer, usecase) = usecase(Ok(Some(now + Duration::days(3))));
        assert!(!usecase.execute(ending(now + Duration::hours(20)), now).await.unwrap());
        assert!(mailer.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn retries_when_the_trial_state_is_unknown() {
        let now = Utc::now();
        let (mailer, usecase) = usecase(Err("down".to_string()));
        assert!(usecase.execute(ending(now + Duration::hours(20)), now).await.is_err());
        assert!(mailer.sent.lock().unwrap().is_empty());
    }
}
