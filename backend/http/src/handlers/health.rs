use actix_web::{HttpResponse, web};
use shared::health::Readiness;

pub async fn livez() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub async fn readyz(readiness: web::Data<Readiness>) -> HttpResponse {
    if readiness.check().await {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::ServiceUnavailable().finish()
    }
}
