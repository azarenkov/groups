use actix_web::web;

use crate::handlers::webhook::livekit_webhook;

pub fn webhook_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/webhook").service(livekit_webhook));
}
