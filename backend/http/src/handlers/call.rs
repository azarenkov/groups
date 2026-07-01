use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use application::dto::requests::call::{
    add_participants::AddCallParticipantsRequest, start_call::StartCallRequest,
};
use domain::user::id::UserId;
use serde::{Deserialize, Serialize};
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::shared::{
    context::request_context, dto::responses::ApiResponse, error::ApiError, state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartCallBody {
    recipient_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddParticipantsBody {
    recipient_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartCallResponse {
    call_id: Uuid,
    token: String,
    url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CallTokenResponse {
    token: String,
    url: String,
}

#[post("")]
async fn start_call(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<StartCallBody>,
) -> Result<impl Responder, ApiError> {
    let caller_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    let result = app_state
        .start_call_usecase
        .execute(
            caller_id,
            StartCallRequest {
                invitee_ids: body.into_inner().recipient_ids,
                ctx,
            },
        )
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(StartCallResponse {
        call_id: result.call_id,
        token: result.token.as_str().to_string(),
        url: result.url,
    })))
}

#[post("/{call_id}/accept")]
async fn accept_call(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    let result = app_state
        .accept_call_usecase
        .execute(user_id, path.into_inner(), ctx.request_id)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(CallTokenResponse {
        token: result.token.as_str().to_string(),
        url: result.url,
    })))
}

#[post("/{call_id}/decline")]
async fn decline_call(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    app_state
        .decline_call_usecase
        .execute(user_id, path.into_inner(), ctx.request_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{call_id}/leave")]
async fn leave_call(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    app_state
        .leave_call_usecase
        .execute(user_id, path.into_inner(), ctx.request_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{call_id}/participants")]
async fn add_call_participants(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<AddParticipantsBody>,
) -> Result<impl Responder, ApiError> {
    let caller_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    app_state
        .add_call_participants_usecase
        .execute(
            caller_id,
            AddCallParticipantsRequest {
                call_id: path.into_inner(),
                invitee_ids: body.into_inner().recipient_ids,
                ctx,
            },
        )
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn call_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/calls")
            .service(start_call)
            .service(accept_call)
            .service(decline_call)
            .service(leave_call)
            .service(add_call_participants),
    );
}
