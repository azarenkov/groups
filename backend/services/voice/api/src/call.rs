use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use voice::dto::requests::call::{
    add_participants::AddCallParticipantsRequest, start_call::StartCallRequest,
};
use domain::user::id::UserId;
use serde::{Deserialize, Serialize};
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::state::AppState;
use api_common::{context::request_context};
use api_common::dto::responses::ApiResponse;
use crate::http_errors::HttpError;

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
struct CallParticipantDto {
    user_id: Uuid,
    user_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartCallResponse {
    call_id: Uuid,
    token: String,
    url: String,
    participants: Vec<CallParticipantDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CallTokenResponse {
    token: String,
    url: String,
    participants: Vec<CallParticipantDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActiveCallWithResponse {
    call_id: Option<Uuid>,
}

#[post("")]
async fn start_call(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<StartCallBody>,
) -> Result<impl Responder, HttpError> {
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
        participants: result
            .participants
            .into_iter()
            .map(|p| CallParticipantDto {
                user_id: p.user_id.uuid(),
                user_name: p.user_name,
            })
            .collect(),
    })))
}

#[get("/active")]
async fn get_active_call(
    _req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let result = app_state
        .get_active_call_usecase
        .execute(user_id)
        .await?
        .map(|r| StartCallResponse {
            call_id: r.call_id,
            token: r.token.as_str().to_string(),
            url: r.url,
            participants: r
                .participants
                .into_iter()
                .map(|p| CallParticipantDto {
                    user_id: p.user_id.uuid(),
                    user_name: p.user_name,
                })
                .collect(),
        });

    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}

#[post("/{call_id}/accept")]
async fn accept_call(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let ctx = request_context(&req);
    let result = app_state
        .accept_call_usecase
        .execute(user_id, path.into_inner(), ctx.request_id)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(CallTokenResponse {
        token: result.token.as_str().to_string(),
        url: result.url,
        participants: result
            .participants
            .into_iter()
            .map(|p| CallParticipantDto {
                user_id: p.user_id.uuid(),
                user_name: p.user_name,
            })
            .collect(),
    })))
}

#[get("/with/{friend_id}")]
async fn active_call_with(
    _req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let call_id = app_state
        .get_active_call_with_usecase
        .execute(user_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ActiveCallWithResponse { call_id })))
}

#[post("/{call_id}/join")]
async fn join_call(
    _req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let result = app_state
        .join_call_usecase
        .execute(user_id, path.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(CallTokenResponse {
        token: result.token.as_str().to_string(),
        url: result.url,
        participants: result
            .participants
            .into_iter()
            .map(|p| CallParticipantDto {
                user_id: p.user_id.uuid(),
                user_name: p.user_name,
            })
            .collect(),
    })))
}

#[post("/{call_id}/decline")]
async fn decline_call(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
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
) -> Result<impl Responder, HttpError> {
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
            .service(get_active_call)
            .service(active_call_with)
            .service(join_call)
            .service(accept_call)
            .service(decline_call)
            .service(leave_call)
            .service(add_call_participants),
    );
}
