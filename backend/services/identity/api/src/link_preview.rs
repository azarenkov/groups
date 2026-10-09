use actix_web::{HttpRequest, HttpResponse, Responder, get, web};
use domain::user::id::UserId;
use uuid::Uuid;

use crate::state::AppState;

const PAGE: &str = r#"<!doctype html>
<html lang="__LANG__">
<head>
<meta charset="utf-8">
<title>__TITLE__</title>
<meta name="robots" content="noindex">
<meta name="description" content="__DESCRIPTION__">
<meta property="og:type" content="__TYPE__">
<meta property="og:site_name" content="Rift">
<meta property="og:title" content="__TITLE__">
<meta property="og:description" content="__DESCRIPTION__">
__LINKS__<meta name="twitter:card" content="summary">
</head>
<body><a href="__HREF__">__TITLE__</a></body>
</html>
"#;

fn escape(value: &str) -> String {
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

fn render(name: Option<&str>, user_id: &str, lang_ru: bool, app_url: &str) -> String {
    let title = name.map(|n| format!("@{n}")).unwrap_or_else(|| "Rift".to_string());
    let description = match (name.is_some(), lang_ru) {
        (true, true) => "Профиль в Rift. Откройте, чтобы добавить в друзья или написать.",
        (true, false) => "Profile on Rift. Open it to add a friend or send a message.",
        (false, true) => "Профиль в Rift",
        (false, false) => "Profile on Rift",
    };
    let base = app_url.trim_end_matches('/');
    let safe_base = (base.starts_with("https://") || base.starts_with("http://")).then_some(base);
    let href = match safe_base {
        Some(b) => format!("{b}/u/{user_id}"),
        None => format!("/u/{user_id}"),
    };
    let links = safe_base
        .map(|b| {
            format!(
                "<meta property=\"og:url\" content=\"{}\">\n<meta property=\"og:image\" content=\"{}\">\n",
                escape(&href),
                escape(&format!("{b}/icons/rift-icon-512.png")),
            )
        })
        .unwrap_or_default();
    PAGE.replace("__LANG__", if lang_ru { "ru" } else { "en" })
        .replace("__TYPE__", if name.is_some() { "profile" } else { "website" })
        .replace("__DESCRIPTION__", description)
        .replace("__LINKS__", &links)
        .replace("__HREF__", &escape(&href))
        .replace("__TITLE__", &escape(&title))
}

#[get("/users/{user_id}")]
pub async fn user_link_preview(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let parsed = Uuid::parse_str(&path.into_inner()).ok();
    let name = match parsed {
        Some(id) => match app_state.get_link_preview_usecase.execute(UserId::new(id)).await {
            Ok(name) => name,
            Err(e) => {
                tracing::warn!(error = %e, "link preview lookup failed");
                None
            }
        },
        None => None,
    };
    let lang_ru = req
        .headers()
        .get("Accept-Language")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim_start().to_ascii_lowercase().starts_with("ru"))
        .unwrap_or(false);
    let user_id = parsed.map(|id| id.to_string()).unwrap_or_default();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .insert_header(("Cache-Control", "public, max-age=300"))
        .insert_header(("Vary", "Accept-Language"))
        .insert_header(("X-Robots-Tag", "noindex"))
        .body(render(name.as_deref(), &user_id, lang_ru, &app_state.public_app_url))
}

#[cfg(test)]
mod tests {
    use super::render;

    const ID: &str = "0b6f3a1e-6a51-4c7e-9a0e-5f1b2c3d4e5f";

    #[test]
    fn a_found_profile_shows_the_name_and_links_back() {
        let html = render(Some("alice"), ID, false, "https://rift.app/");
        assert!(html.contains("<meta property=\"og:title\" content=\"@alice\">"));
        assert!(html.contains("<meta property=\"og:url\" content=\"https://rift.app/u/0b6f3a1e-6a51-4c7e-9a0e-5f1b2c3d4e5f\">"));
        assert!(html.contains("<meta property=\"og:image\" content=\"https://rift.app/icons/rift-icon-512.png\">"));
        assert!(html.contains("content=\"profile\""));
        assert!(html.contains("<html lang=\"en\">"));
    }

    #[test]
    fn a_missing_profile_looks_like_plain_rift() {
        let html = render(None, ID, true, "https://rift.app");
        assert!(html.contains("<title>Rift</title>"));
        assert!(html.contains("content=\"Профиль в Rift\""));
        assert!(html.contains("content=\"website\""));
    }

    #[test]
    fn names_are_escaped() {
        let html = render(Some("a\"><script>"), ID, false, "https://rift.app");
        assert!(!html.contains("<script>"));
        assert!(html.contains("@a&quot;&gt;&lt;script&gt;"));
    }

    #[test]
    fn non_http_app_urls_are_not_linked() {
        let html = render(Some("alice"), ID, false, "javascript:alert(1)");
        assert!(!html.contains("og:url"));
        assert!(!html.contains("javascript:"));
    }
}
