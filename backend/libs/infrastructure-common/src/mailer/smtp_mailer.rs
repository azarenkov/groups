use crate::config::mailer::MailerConfig;
use domain::{user::email::Email, user::locale::Locale};
use ports::{mailer::CodePurpose, mailer::Mailer, mailer::MailerError};
use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use urlencoding::encode;

use super::templates::{self, RenderedEmail};

const SMTP_TIMEOUT: Duration = Duration::from_secs(15);

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
            .timeout(Some(SMTP_TIMEOUT))
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

    async fn deliver(&self, to: &Email, rendered: RenderedEmail) -> Result<(), MailerError> {
        let mailer = self.clone();
        let to = to.clone();
        tokio::task::spawn_blocking(move || mailer.send(&to, rendered))
            .await
            .map_err(|e| MailerError::Sending(format!("mail task failed: {}", e)))?
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
            .multipart(body(rendered.text, rendered.html))
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

fn body(text: String, html: String) -> MultiPart {
    let logo = Attachment::new_inline(templates::LOGO_CID.to_string())
        .body(templates::LOGO_PNG.to_vec(), ContentType::parse("image/png").expect("static content type"));
    MultiPart::alternative()
        .singlepart(SinglePart::plain(text))
        .multipart(
            MultiPart::related()
                .singlepart(SinglePart::html(html))
                .singlepart(logo),
        )
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
        self.deliver(email, templates::password_reset(&reset_link, locale, self.inner.app_url.as_deref()))
            .await
    }

    async fn send_verification_code(
        &self,
        email: &Email,
        verification_code: &str,
        purpose: CodePurpose,
        locale: Locale,
    ) -> Result<(), MailerError> {
        self.deliver(
            email,
            templates::verification_code(verification_code, purpose, locale, self.inner.app_url.as_deref()),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logo_travels_inside_the_letter() {
        let rendered = templates::password_reset("https://rift.app/reset-password?token=t", Locale::En, None);
        assert!(rendered.html.contains("src=\"cid:rift-logo\""));
        let message = Message::builder()
            .from("Rift <noreply@rift.app>".parse().unwrap())
            .to("user@example.com".parse().unwrap())
            .subject(rendered.subject)
            .multipart(body(rendered.text, rendered.html))
            .unwrap();
        let raw = String::from_utf8(message.formatted()).unwrap();
        assert!(raw.contains("multipart/alternative"));
        assert!(raw.contains("multipart/related"));
        assert!(raw.contains("Content-ID: <rift-logo>"));
        assert!(raw.contains("Content-Type: image/png"));
        assert!(raw.contains("Content-Disposition: inline"));
    }
}
