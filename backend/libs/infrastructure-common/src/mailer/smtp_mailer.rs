use crate::config::mailer::MailerConfig;
use domain::{user::email::Email, user::locale::Locale};
use ports::{mailer::Mailer, mailer::MailerError};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Utc};
use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::sync::{Arc, Mutex};

use urlencoding::encode;

#[derive(Clone)]
pub struct SmtpMailer {
    inner: Arc<Inner>,
}

struct Inner {
    mailer: Mutex<SmtpTransport>,
    from_mailbox: Mailbox,
    reset_base_url: String,
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
            }),
        })
    }

    fn send_html(
        &self,
        to: &Email,
        subject: &str,
        body: String,
    ) -> Result<(), MailerError> {
        let to_mailbox: Mailbox = to
            .as_str()
            .parse()
            .map_err(|e| MailerError::Sending(format!("invalid recipient address: {}", e)))?;
        let email = Message::builder()
            .from(self.inner.from_mailbox.clone())
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body)
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

        let (subject, html_body) = match locale {
            Locale::Ru => (
                "Сброс пароля",
                format!(
                    r#"<p>Здравствуйте,</p>
                    <p>Вы запросили сброс пароля. Перейдите по ссылке ниже:</p>
                    <p><a href="{link}">{link}</a></p>
                    <p>Если вы не запрашивали сброс пароля, проигнорируйте это сообщение.</p>"#,
                    link = reset_link
                ),
            ),
            Locale::En => (
                "Password reset",
                format!(
                    r#"<p>Hello,</p>
                    <p>You requested a password reset. Follow the link below:</p>
                    <p><a href="{link}">{link}</a></p>
                    <p>If you did not request a reset, please ignore this message.</p>"#,
                    link = reset_link
                ),
            ),
        };

        self.send_html(email, subject, html_body)
    }

    async fn send_verification_code(
        &self,
        email: &Email,
        verification_code: &str,
        locale: Locale,
    ) -> Result<(), MailerError> {
        let (subject, html_body) = match locale {
            Locale::Ru => (
                "Код подтверждения",
                format!(
                    r#"<p>Здравствуйте,</p>
                    <p>Ваш код подтверждения:</p>
                    <p style="font-size: 24px; font-weight: bold; letter-spacing: 4px;">{code}</p>
                    <p>Если вы не запрашивали этот код, проигнорируйте это сообщение.</p>"#,
                    code = verification_code
                ),
            ),
            Locale::En => (
                "Verification code",
                format!(
                    r#"<p>Hello,</p>
                    <p>Your verification code:</p>
                    <p style="font-size: 24px; font-weight: bold; letter-spacing: 4px;">{code}</p>
                    <p>If you did not request this code, please ignore this message.</p>"#,
                    code = verification_code
                ),
            ),
        };

        self.send_html(email, subject, html_body)
    }

    async fn send_trial_ending(
        &self,
        email: &Email,
        ends_at: DateTime<Utc>,
        locale: Locale,
    ) -> Result<(), MailerError> {
        let app_link = self.inner.reset_base_url.trim_end_matches('/').to_string();
        let date = trial_end_date(ends_at, locale);
        let (subject, html_body) = match locale {
            Locale::Ru => (
                "Пробный Rift Pro заканчивается завтра",
                format!(
                    r#"<p>Здравствуйте,</p>
                    <p>Пробный период Rift Pro закончится {date}.</p>
                    <p>После этого аккаунт вернётся на Free. Баннер и анимированный аватар скроются, но не удалятся, а сообщения и вложения останутся на месте.</p>
                    <p><a href="{link}">{link}</a></p>"#,
                    date = date,
                    link = app_link
                ),
            ),
            Locale::En => (
                "Your Rift Pro trial ends tomorrow",
                format!(
                    r#"<p>Hello,</p>
                    <p>Your Rift Pro trial ends on {date}.</p>
                    <p>After that the account goes back to Free. Your banner and animated avatar are hidden but kept, and messages and attachments stay as they are.</p>
                    <p><a href="{link}">{link}</a></p>"#,
                    date = date,
                    link = app_link
                ),
            ),
        };

        self.send_html(email, subject, html_body)
    }
}

pub(crate) fn trial_end_date(ends_at: DateTime<Utc>, locale: Locale) -> String {
    const RU_MONTHS: [&str; 12] = [
        "января", "февраля", "марта", "апреля", "мая", "июня",
        "июля", "августа", "сентября", "октября", "ноября", "декабря",
    ];
    let month = ends_at.month0() as usize;
    match locale {
        Locale::Ru => format!("{} {} в {} UTC", ends_at.day(), RU_MONTHS[month], ends_at.format("%H:%M")),
        Locale::En => format!("{} at {} UTC", ends_at.format("%B %-d"), ends_at.format("%H:%M")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn trial_end_date_reads_naturally_in_both_languages() {
        let at = Utc.with_ymd_and_hms(2026, 10, 6, 9, 5, 0).unwrap();
        assert_eq!(trial_end_date(at, Locale::Ru), "6 октября в 09:05 UTC");
        assert_eq!(trial_end_date(at, Locale::En), "October 6 at 09:05 UTC");
    }
}
