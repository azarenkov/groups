use chrono::{DateTime, Datelike, Utc};
use domain::user::locale::Locale;
use ports::mailer::CodePurpose;

pub const LOGO_CID: &str = "rift-logo";
pub const LOGO_PNG: &[u8] = include_bytes!("assets/rift-logo.png");

const CANVAS: &str = "#000000";
const CARD: &str = "#121212";
const RAISED: &str = "#242424";
const FG_1: &str = "#f5f5f7";
const FG_2: &str = "#c7c7cc";
const FG_3: &str = "#8e8e93";
const FONT: &str = "Inter,-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif";
const MONO: &str = "'JetBrains Mono',SFMono-Regular,Menlo,Consolas,monospace";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEmail {
    pub subject: String,
    pub html: String,
    pub text: String,
}

enum Action<'a> {
    Code(&'a str),
    Button { label: &'a str, url: &'a str, fallback: &'a str },
}

struct Letter<'a> {
    locale: Locale,
    subject: String,
    preheader: String,
    title: String,
    lead: Vec<String>,
    action: Option<Action<'a>>,
    notes: Vec<String>,
    reason: &'a str,
}

pub fn verification_code(code: &str, purpose: CodePurpose, locale: Locale, app_url: Option<&str>) -> RenderedEmail {
    let letter = match (purpose, locale) {
        (CodePurpose::Registration, Locale::Ru) => Letter {
            locale,
            subject: format!("{code} — код подтверждения Rift"),
            preheader: "Введите код в Rift, чтобы закончить регистрацию.".into(),
            title: "Подтвердите почту".into(),
            lead: vec!["Введите этот код в Rift, чтобы закончить регистрацию.".into()],
            action: Some(Action::Code(code)),
            notes: vec!["Код действует 30 минут. Если вы не регистрировались в Rift, просто проигнорируйте письмо.".into()],
            reason: "Вы получили это письмо, потому что этот адрес указали при регистрации в Rift.",
        },
        (CodePurpose::Registration, Locale::En) => Letter {
            locale,
            subject: format!("{code} is your Rift verification code"),
            preheader: "Enter the code in Rift to finish signing up.".into(),
            title: "Confirm your email".into(),
            lead: vec!["Enter this code in Rift to finish creating your account.".into()],
            action: Some(Action::Code(code)),
            notes: vec!["The code works for 30 minutes. If you didn't sign up for Rift, you can ignore this email.".into()],
            reason: "You're getting this email because this address was used to sign up for Rift.",
        },
        (CodePurpose::EmailChange, Locale::Ru) => Letter {
            locale,
            subject: format!("{code} — код для смены почты в Rift"),
            preheader: "Введите код в настройках аккаунта, чтобы сменить почту.".into(),
            title: "Подтвердите новый адрес".into(),
            lead: vec!["Введите этот код в настройках аккаунта Rift, чтобы сменить почту на этот адрес.".into()],
            action: Some(Action::Code(code)),
            notes: vec!["Код действует 10 минут. Если вы не меняли почту, проигнорируйте письмо — адрес останется прежним.".into()],
            reason: "Вы получили это письмо, потому что этот адрес указали как новую почту аккаунта Rift.",
        },
        (CodePurpose::EmailChange, Locale::En) => Letter {
            locale,
            subject: format!("{code} is your Rift email change code"),
            preheader: "Enter the code in your account settings to change your email.".into(),
            title: "Confirm your new address".into(),
            lead: vec!["Enter this code in your Rift account settings to switch your email to this address.".into()],
            action: Some(Action::Code(code)),
            notes: vec!["The code works for 10 minutes. If you didn't ask to change your email, ignore this message and nothing will change.".into()],
            reason: "You're getting this email because this address was entered as the new email of a Rift account.",
        },
    };
    render(letter, app_url)
}

pub fn password_reset(link: &str, locale: Locale, app_url: Option<&str>) -> RenderedEmail {
    let letter = match locale {
        Locale::Ru => Letter {
            locale,
            subject: "Сброс пароля Rift".into(),
            preheader: "Ссылка для нового пароля действует 30 минут.".into(),
            title: "Сброс пароля".into(),
            lead: vec!["Мы получили запрос на сброс пароля вашего аккаунта Rift. Нажмите кнопку, чтобы задать новый.".into()],
            action: Some(Action::Button {
                label: "Задать новый пароль",
                url: link,
                fallback: "Если кнопка не открывается, скопируйте ссылку в браузер:",
            }),
            notes: vec![
                "Ссылка действует 30 минут и сработает один раз. После смены пароля все сеансы завершатся.".into(),
                "Если вы не запрашивали сброс, просто проигнорируйте письмо — пароль останется прежним.".into(),
            ],
            reason: "Вы получили это письмо, потому что для этого адреса запросили сброс пароля в Rift.",
        },
        Locale::En => Letter {
            locale,
            subject: "Reset your Rift password".into(),
            preheader: "The link to set a new password works for 30 minutes.".into(),
            title: "Reset your password".into(),
            lead: vec!["We got a request to reset the password of your Rift account. Press the button to choose a new one.".into()],
            action: Some(Action::Button {
                label: "Set a new password",
                url: link,
                fallback: "If the button doesn't open, copy this link into your browser:",
            }),
            notes: vec![
                "The link works for 30 minutes and only once. Changing the password signs you out everywhere.".into(),
                "If you didn't ask for a reset, ignore this email and your password stays the same.".into(),
            ],
            reason: "You're getting this email because a password reset was requested for this address on Rift.",
        },
    };
    render(letter, app_url)
}

pub fn trial_ending(ends_at: DateTime<Utc>, locale: Locale, app_url: Option<&str>) -> RenderedEmail {
    let date = trial_end_date(ends_at, locale);
    let letter = match locale {
        Locale::Ru => Letter {
            locale,
            subject: "Пробный Rift Pro заканчивается завтра".into(),
            preheader: format!("Пробный период закончится {date}."),
            title: "Пробный Rift Pro заканчивается".into(),
            lead: vec![format!("Пробный период Rift Pro закончится {date}.")],
            action: app_url.map(|url| Action::Button {
                label: "Открыть Rift",
                url,
                fallback: "Или откройте ссылку:",
            }),
            notes: vec!["После этого аккаунт вернётся на Free. Баннер и анимированный аватар скроются, но не удалятся, а сообщения и вложения останутся на месте.".into()],
            reason: "Вы получили это письмо, потому что у вашего аккаунта Rift подключён пробный Pro.",
        },
        Locale::En => Letter {
            locale,
            subject: "Your Rift Pro trial ends tomorrow".into(),
            preheader: format!("Your trial ends on {date}."),
            title: "Your Rift Pro trial is ending".into(),
            lead: vec![format!("Your Rift Pro trial ends on {date}.")],
            action: app_url.map(|url| Action::Button {
                label: "Open Rift",
                url,
                fallback: "Or open this link:",
            }),
            notes: vec!["After that the account goes back to Free. Your banner and animated avatar are hidden but kept, and messages and attachments stay as they are.".into()],
            reason: "You're getting this email because your Rift account has a Pro trial.",
        },
    };
    render(letter, app_url)
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

pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn safe_url(url: &str) -> Option<&str> {
    let url = url.trim();
    (url.starts_with("https://") || url.starts_with("http://")).then_some(url)
}

fn render(letter: Letter<'_>, app_url: Option<&str>) -> RenderedEmail {
    let app_url = app_url.and_then(safe_url).map(|u| u.trim_end_matches('/'));
    let lang = match letter.locale {
        Locale::Ru => "ru",
        Locale::En => "en",
    };
    let paragraph = |text: &str, color: &str, size: u8| {
        format!(
            r#"<p style="margin:0 0 16px;font-family:{FONT};font-size:{size}px;line-height:1.6;color:{color};">{}</p>"#,
            escape(text)
        )
    };

    let logo = format!(
        r#"<img src="cid:{LOGO_CID}" width="44" height="44" alt="Rift" style="display:block;margin:0 auto;border:0;border-radius:10px;">"#
    );

    let action_html = match &letter.action {
        Some(Action::Code(code)) => format!(
            r#"<table role="presentation" cellpadding="0" cellspacing="0" border="0" style="margin:8px auto 28px;"><tr><td align="center" bgcolor="{RAISED}" style="background:{RAISED};border-radius:8px;padding:20px 32px;font-family:{MONO};font-size:34px;line-height:1;font-weight:700;letter-spacing:10px;color:{FG_1};">{}</td></tr></table>"#,
            escape(code)
        ),
        Some(Action::Button { label, url, fallback }) => match safe_url(url) {
            Some(url) => format!(
                r#"<table role="presentation" cellpadding="0" cellspacing="0" border="0" style="margin:8px auto 24px;"><tr><td align="center" bgcolor="{FG_1}" style="background:{FG_1};border-radius:999px;"><a href="{href}" target="_blank" style="display:inline-block;padding:15px 36px;font-family:{FONT};font-size:15px;line-height:1;font-weight:700;color:#000000;text-decoration:none;border-radius:999px;">{label}</a></td></tr></table><p style="margin:0 0 20px;font-family:{FONT};font-size:12px;line-height:1.6;color:{FG_3};text-align:center;">{fallback}<br><a href="{href}" target="_blank" style="color:{FG_2};text-decoration:underline;word-break:break-all;">{href}</a></p>"#,
                href = escape(url),
                label = escape(label),
                fallback = escape(fallback),
            ),
            None => String::new(),
        },
        None => String::new(),
    };

    let footer_link = match app_url {
        Some(url) => format!(
            r#" · <a href="{href}" target="_blank" style="color:{FG_3};text-decoration:underline;">{host}</a>"#,
            href = escape(url),
            host = escape(url.split("://").nth(1).unwrap_or(url)),
        ),
        None => String::new(),
    };

    let lead: String = letter.lead.iter().map(|p| paragraph(p, FG_2, 15)).collect();
    let notes: String = letter.notes.iter().map(|p| paragraph(p, FG_3, 13)).collect();

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark light">
<meta name="supported-color-schemes" content="dark light">
<title>{subject}</title>
</head>
<body style="margin:0;padding:0;background:{CANVAS};" bgcolor="{CANVAS}">
<div style="display:none;max-height:0;max-width:0;overflow:hidden;opacity:0;mso-hide:all;">{preheader}</div>
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" bgcolor="{CANVAS}" style="background:{CANVAS};">
<tr><td align="center" style="padding:32px 16px;">
<table role="presentation" width="560" cellpadding="0" cellspacing="0" border="0" style="width:100%;max-width:560px;">
<tr><td align="center" style="padding:8px 4px 28px;">{logo}</td></tr>
<tr><td bgcolor="{CARD}" align="center" style="background:{CARD};border-radius:8px;padding:44px 40px 24px;text-align:center;">
<h1 style="margin:0 0 16px;font-family:{FONT};font-size:30px;line-height:1.2;font-weight:800;letter-spacing:-0.6px;color:{FG_1};">{title}</h1>
{lead}{action_html}{notes}</td></tr>
<tr><td align="center" style="padding:24px 16px 0;font-family:{FONT};font-size:12px;line-height:1.6;color:{FG_3};text-align:center;">{reason}<br>Rift{footer_link}</td></tr>
</table>
</td></tr>
</table>
</body>
</html>
"#,
        subject = escape(&letter.subject),
        preheader = escape(&letter.preheader),
        title = escape(&letter.title),
        reason = escape(letter.reason),
    );

    let mut text = format!("{}\n\n", letter.title);
    for p in &letter.lead {
        text.push_str(p);
        text.push_str("\n\n");
    }
    match &letter.action {
        Some(Action::Code(code)) => text.push_str(&format!("    {code}\n\n")),
        Some(Action::Button { label, url, .. }) => {
            if let Some(url) = safe_url(url) {
                text.push_str(&format!("{label}: {url}\n\n"));
            }
        }
        None => {}
    }
    for p in &letter.notes {
        text.push_str(p);
        text.push_str("\n\n");
    }
    text.push_str("--\n");
    text.push_str(letter.reason);
    text.push('\n');
    text.push_str(&match app_url {
        Some(url) => format!("Rift · {url}\n"),
        None => "Rift\n".to_string(),
    });

    RenderedEmail {
        subject: letter.subject,
        html,
        text,
    }
}

#[cfg(test)]
#[path = "templates_tests.rs"]
mod tests;
