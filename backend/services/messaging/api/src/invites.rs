use actix_web::{HttpResponse, Responder, get, post, web};
use api_common::api_error::StatusCode;
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use domain::user::id::UserId;
use messaging_domain::server::invite_link::InviteToken;

use crate::dto::responses::server::invite_links::{InvitePreviewResponse, JoinInviteResponse};
use crate::http_errors::HttpError;
use crate::state::AppState;

fn parse_token(raw: &str) -> Result<InviteToken, HttpError> {
    InviteToken::parse(raw).ok_or_else(|| {
        HttpError::new(
            "This invite link is invalid.".to_string(),
            StatusCode::NOT_FOUND,
            "INVITE_NOT_FOUND",
        )
    })
}

#[get("/{token}")]
async fn preview_invite(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, HttpError> {
    let token = parse_token(&path.into_inner())?;
    let preview = app_state.preview_invite_link_usecase.execute(token).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(InvitePreviewResponse::from(preview))))
}

#[post("/{token}/join")]
async fn join_invite(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder, HttpError> {
    let token = parse_token(&path.into_inner())?;
    let joined = app_state
        .join_by_invite_link_usecase
        .execute(token, UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(JoinInviteResponse::from(joined))))
}

pub fn invite_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/invites")
            .service(preview_invite)
            .service(join_invite),
    );
}
