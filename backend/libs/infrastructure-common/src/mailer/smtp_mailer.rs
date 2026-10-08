use crate::config::mailer::MailerConfig;
use domain::{user::email::Email, user::locale::Locale};
use ports::{mailer::CodePurpose, mailer::Mailer, mailer::MailerError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::sync::{Arc, Mutex};

use urlencoding::encode;

use super::templates::{self, RenderedEmail};

#[derive(Clone)]
pub struct SmtpMailer {
    inner: Arc<Inner>,
}

struct Inner {
    mailer: Mutex<SmtpTransport>,
    from_mailbox: Mailbox,
    reset_base_url: String,
    app_url: Option<String>,
}

impl SmtpMailer {
    pub fn from_config(config: MailerConfig) -> Result<Self, MailerError> {
        let from_mailbox: Mailbox = config
            .from
            .parse()
            .map_err(|e| MailerError::Sending(format!("invalid MAILER_FROM: {}", e)))?;

        let transport = SmtpTransport::relay(&config.host)
            .map_err(|e| MailerError::Sending(format!("failed to create relay: {}", e)))?
            .credentials(Credentials::new(config.username, config.password))
            .port(config.port)
            .build();

        Ok(Self {
            inner: Arc::new(Inner {
                mailer: Mutex::new(transport),
                from_mailbox,
                reset_base_url: config.password_reset_base_url,
                app_url: config
                    .app_url
                    .map(|url| url.trim().trim_end_matches('/').to_string())
                    .filter(|url| !url.is_empty()),
            }),
        })
    }

    fn send(&self, to: &Email, rendered: RenderedEmail) -> Result<(), MailerError> {
        let to_mailbox: Mailbox = to
            .as_str()
            .parse()
            .map_err(|e| MailerError::Sending(format!("invalid recipient address: {}", e)))?;
        let email = Message::builder()
            .from(self.inner.from_mailbox.clone())
            .to(to_mailbox)
            .subject(rendered.subject)
            .multipart(MultiPart::alternative_plain_html(rendered.text, rendered.html))
            .map_err(|e| MailerError::Sending(format!("failed to build message: {}", e)))?;
        let mailer = self
            .inner
            .mailer
            .lock()
            .map_err(|e| MailerError::Sending(format!("mutex poisoned: {}", e)))?;
        mailer
            .send(&email)
            .map_err(|e| MailerError::Sending(format!("failed to send email: {}", e)))?;
        Ok(())
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send_password_reset(
        &self,
        email: &Email,
        token: &str,
        locale: Locale,
    ) -> Result<(), MailerError> {
        let reset_link = format!(
            "{}/reset-password?token={}",
            self.inner.reset_base_url.trim_end_matches('/'),
            encode(token).into_owned()
        );
        self.send(email, templates::password_reset(&reset_link, locale, self.inner.app_url.as_deref()))
    }

    async fn send_verification_code(
        &self,
        email: &Email,
        verification_code: &str,
        purpose: CodePurpose,
        locale: Locale,
    ) -> Result<(), MailerError> {
        self.send(
            email,
            templates::verification_code(verification_code, purpose, locale, self.inner.app_url.as_deref()),
        )
    }

    async fn send_trial_ending(
        &self,
        email: &Email,
        ends_at: DateTime<Utc>,
        locale: Locale,
    ) -> Result<(), MailerError> {
        self.send(email, templates::trial_ending(ends_at, locale, self.inner.app_url.as_deref()))
    }
}
