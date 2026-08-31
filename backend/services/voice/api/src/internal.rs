use actix_web::{HttpResponse, Responder, get, web};
use contracts::internal::calls::CallParticipationResponseV1;
use domain::user::id::UserId;
use uuid::Uuid;
use voice_domain::call::repository::CallRepository;

use crate::http_errors::HttpError;
use crate::state::AppState;

#[get("/calls/{call_id}/participation/{a}/{b}")]
async fn call_participation(
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let (call_id, a, b) = path.into_inner();
    let participated = app_state
        .call_repository
        .pair_participated(call_id, &UserId::new(a), &UserId::new(b))
        .await?;
    Ok(HttpResponse::Ok().json(CallParticipationResponseV1 { participated }))
}

pub fn internal_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/internal").service(call_participation));
}
