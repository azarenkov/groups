use actix_web::{HttpResponse, Responder, post, web};
use api_common::middleware::auth::RateLimitedAuthToken;
use domain::user::id::UserId;
use messaging::usecases::report::create_report::CreateReportRequest;
use messaging_domain::report::{ReportContext, ReportTarget};
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateReportBody {
    reason: String,
    details: Option<String>,
    channel_id: Option<Uuid>,
    message_id: Option<Uuid>,
}

impl CreateReportBody {
    fn into_request(self, target: ReportTarget, target_id: Uuid) -> CreateReportRequest {
        CreateReportRequest {
            target,
            target_id,
            reason: self.reason,
            details: self.details,
            context: ReportContext {
                channel_id: self.channel_id,
                message_id: self.message_id,
            },
        }
    }
}

#[post("/{user_id}/report")]
async fn report_user(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<CreateReportBody>,
) -> Result<impl Responder, HttpError> {
    let request = body
        .into_inner()
        .into_request(ReportTarget::User, path.into_inner());
    app_state
        .create_report_usecase
        .execute(UserId::new(auth_token.id), request)
        .await?;
    Ok(HttpResponse::Accepted().finish())
}

#[post("/{server_id}/report")]
async fn report_server(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<CreateReportBody>,
) -> Result<impl Responder, HttpError> {
    let request = body
        .into_inner()
        .into_request(ReportTarget::Server, path.into_inner());
    app_state
        .create_report_usecase
        .execute(UserId::new(auth_token.id), request)
        .await?;
    Ok(HttpResponse::Accepted().finish())
}

pub fn user_report_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(report_user);
}

pub fn server_report_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(report_server);
}
