use actix_web::{HttpResponse, Responder, get, post, web};
use contracts::internal::calls::{CallInvitationV1, CallOutcomeResponseV1, CallRoomsRequestV1, CallRoomsResponseV1, EndedCallResponseV1};
use domain::user::id::UserId;
use uuid::Uuid;
use voice_domain::call::repository::CallRepository;

use crate::http_errors::HttpError;
use crate::state::AppState;

#[get("/calls/{call_id}/outcome/{caller}/{callee}")]
async fn call_outcome(
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (call_id, caller, callee) = path.into_inner();
    let outcome = app_state
        .call_repository
        .pair_outcome(call_id, &UserId::new(caller), &UserId::new(callee))
        .await?;
    Ok(HttpResponse::Ok().json(CallOutcomeResponseV1 {
        participated: outcome.participated,
        caller_in_call: outcome.caller_in_call,
        caller_left_at: outcome.caller_left_at,
        callee_left_at: outcome.callee_left_at,
        callee_invited_by_caller: outcome.callee_invited_by_caller,
        callee_pending: outcome.callee_pending,
        callee_answered_at: outcome.callee_answered_at,
        callee_declined: outcome.callee_declined,
    }))
}

#[get("/calls/{call_id}/ended")]
async fn ended_call(app_state: web::Data<AppState>, path: web::Path<Uuid>) -> Result<impl Responder, HttpError> {
    let ended = app_state.call_repository.ended_call(path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ended.map(|call| EndedCallResponseV1 {
        ended_at: call.ended_at,
        invitations: call
            .invitations
            .iter()
            .map(|i| CallInvitationV1 { inviter_id: i.inviter.uuid(), invitee_id: i.invitee.uuid() })
            .collect(),
    })))
}

#[post("/calls/rooms")]
async fn call_rooms(app_state: web::Data<AppState>, body: web::Json<CallRoomsRequestV1>) -> Result<impl Responder, HttpError> {
    let call_ids = app_state.call_repository.call_rooms_among(&body.ids).await?;
    Ok(HttpResponse::Ok().json(CallRoomsResponseV1 { call_ids }))
}

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/internal").service(call_outcome).service(ended_call).service(call_rooms));
}
