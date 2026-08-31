use actix_web::{HttpResponse, Responder, get, web};
use contracts::internal::calls::CallOutcomeResponseV1;
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
        callee_invited_by_caller: outcome.callee_invited_by_caller,
        callee_pending: outcome.callee_pending,
        callee_answered_at: outcome.callee_answered_at,
        callee_declined: outcome.callee_declined,
    }))
}

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/internal").service(call_outcome));
}
