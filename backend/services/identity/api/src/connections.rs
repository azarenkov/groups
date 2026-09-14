use std::collections::HashMap;

use actix_web::{HttpRequest, HttpResponse, Responder, delete, get, post, web};
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use identity_domain::user::id::UserId;
use serde::Serialize;
use uuid::Uuid;

use crate::http_errors::HttpError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionItem {
    provider: String,
    display_name: String,
    url: Option<String>,
}

fn connection_url(provider: &str, subject: &str, display_name: &str) -> Option<String> {
    match provider {
        "github" if !display_name.is_empty() => {
            Some(format!("https://github.com/{display_name}"))
        }
        "steam" => Some(format!("https://steamcommunity.com/profiles/{subject}")),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
struct ConnectionsResponse {
    connections: Vec<ConnectionItem>,
}

#[derive(Debug, Serialize)]
struct StartConnectionResponse {
    url: String,
}

#[get("/me/connections")]
pub async fn list_connections(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let items = app_state
        .list_connections_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ConnectionsResponse {
        connections: items
            .into_iter()
            .map(|c| ConnectionItem {
                url: connection_url(&c.provider, &c.subject, &c.display_name),
                provider: c.provider,
                display_name: c.display_name,
            })
            .collect(),
    })))
}

#[get("/{user_id}/connections")]
pub async fn user_connections(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let items = app_state
        .list_user_connections_usecase
        .execute(UserId::new(auth_token.id), UserId::new(path.into_inner()))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ConnectionsResponse {
        connections: items
            .into_iter()
            .map(|c| ConnectionItem {
                url: connection_url(&c.provider, &c.subject, &c.display_name),
                provider: c.provider,
                display_name: c.display_name,
            })
            .collect(),
    })))
}

#[post("/me/connections/{provider}/start")]
pub async fn start_connection(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, HttpError> {
    let url = app_state
        .start_connection_usecase
        .execute(UserId::new(auth_token.id), &path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(StartConnectionResponse { url })))
}

#[delete("/me/connections/{provider}")]
pub async fn remove_connection(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, HttpError> {
    app_state
        .remove_connection_usecase
        .execute(UserId::new(auth_token.id), &path.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

const RESULT_PAGE: &str = r##"<!doctype html>
<html lang="__LANG__"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>__TITLE__</title><style>
*{box-sizing:border-box}
body{margin:0;min-height:100vh;display:flex;flex-direction:column;
background:#0d0d0f;color:#f5f5f7;
font:15px/1.5 -apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif}
.topbar{height:52px;display:flex;align-items:center;padding:0 24px;flex-shrink:0}
.brand{display:inline-flex;align-items:center;gap:8px;font-size:14px;font-weight:600;
letter-spacing:-0.005em;color:#f5f5f7}
.wrap{flex:1;display:flex;padding:24px}
.card{width:360px;max-width:100%;margin:auto;text-align:center}
.badge{width:56px;height:56px;margin:0 auto 20px;border-radius:50%;display:flex;
align-items:center;justify-content:center;font-size:26px;font-weight:700;
background:__BADGE_BG__;color:__BADGE_FG__}
h1{font-size:26px;font-weight:600;letter-spacing:-0.022em;margin:0 0 6px}
.sub{font-size:14px;color:#8e8e93;margin:0}
a.btn{display:block;margin-top:28px;padding:12px;border-radius:10px;background:#0a84ff;
color:#fff;text-decoration:none;font-size:14px;font-weight:600;
transition:background 120ms ease}
a.btn:hover{background:#1a90ff}
</style></head><body>
<div class="topbar"><span class="brand">
<svg width="22" height="22" viewBox="0 0 48 48" fill="none">
<mask id="gap"><rect width="48" height="48" fill="white"/>
<rect x="22.5" y="0" width="3" height="48" fill="black"/></mask>
<g mask="url(#gap)" stroke="#f5f5f7" fill="none">
<circle cx="24" cy="24" r="14" stroke-width="3"/>
<circle cx="24" cy="24" r="7.5" stroke-width="1.75" opacity="0.7"/>
</g></svg>
<span>Rift</span>
</span></div>
<div class="wrap"><div class="card">
<div class="badge">__BADGE__</div>
<h1>__TITLE__</h1><p class="sub">__BODY__</p>
__ACTION__
</div></div></body></html>"##;

fn provider_title(provider: &str) -> &str {
    match provider {
        "github" => "GitHub",
        "steam" => "Steam",
        _ => "Account",
    }
}

fn result_page(lang_ru: bool, ok: bool, provider: &str, app_href: Option<&str>) -> String {
    let provider = provider_title(provider);
    let (title, body, button) = match (lang_ru, ok) {
        (true, true) => (
            "Аккаунт привязан".to_string(),
            format!("{provider} подключён к вашему профилю Rift. Эту вкладку можно закрыть."),
            "Открыть Rift",
        ),
        (true, false) => (
            "Не удалось привязать аккаунт".to_string(),
            "Попробуйте ещё раз из настроек Rift (Настройки → Подключения).".to_string(),
            "Открыть Rift",
        ),
        (false, true) => (
            "Account linked".to_string(),
            format!("{provider} is now connected to your Rift profile. You can close this tab."),
            "Open Rift",
        ),
        (false, false) => (
            "Couldn't link the account".to_string(),
            "Please try again from Rift settings (Settings → Connections).".to_string(),
            "Open Rift",
        ),
    };
    RESULT_PAGE
        .replace("__LANG__", if lang_ru { "ru" } else { "en" })
        .replace(
            "__BADGE_BG__",
            if ok { "rgba(48,209,88,0.15)" } else { "rgba(255,69,58,0.15)" },
        )
        .replace("__BADGE_FG__", if ok { "#30d158" } else { "#ff453a" })
        .replace("__BADGE__", if ok { "&#10003;" } else { "!" })
        .replace("__TITLE__", &title)
        .replace("__BODY__", &body)
        .replace(
            "__ACTION__",
            &app_href
                .map(|href| format!("<a class=\"btn\" href=\"{href}\">{button}</a>"))
                .unwrap_or_default(),
        )
}

#[get("/{provider}/callback")]
pub async fn connection_callback(
    req: HttpRequest,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let provider = path.into_inner();
    let ok = match app_state
        .complete_connection_usecase
        .execute(&provider, &query)
        .await
    {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(error = %e, provider = %provider, "connection callback failed");
            false
        }
    };
    let lang_ru = req
        .headers()
        .get("Accept-Language")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_ascii_lowercase().starts_with("ru"))
        .unwrap_or(false);
    let app_href = (!app_state.public_app_url.is_empty())
        .then(|| format!("{}/", app_state.public_app_url.trim_end_matches('/')));
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(result_page(lang_ru, ok, &provider, app_href.as_deref()))
}
