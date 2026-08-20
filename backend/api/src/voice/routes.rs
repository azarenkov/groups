use actix_web::web;

use crate::shared::health::{livez, readyz};
use crate::voice::call::call_routes;
use crate::voice::webhook::livekit_webhook;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/livez").to(livez))
        .service(web::resource("/readyz").to(readyz))
        .service(
            web::scope("/api")
                .configure(call_routes)
                .service(web::scope("/webhook").service(livekit_webhook)),
        );
}
