use actix_web::web;

use api_common::health::{livez, readyz};
use crate::call::call_routes;
use crate::webhook::livekit_webhook;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/livez").to(livez))
        .service(web::resource("/readyz").to(readyz))
        .configure(crate::internal::internal_routes)
        .service(
            web::scope("/api")
                .configure(call_routes)
                .service(web::scope("/webhook").service(livekit_webhook)),
        );
}
