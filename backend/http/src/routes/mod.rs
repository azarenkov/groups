use actix_web::web;

use crate::handlers::health::{livez, readyz};

pub mod auth;
pub mod channel;
pub mod server;
pub mod webhook;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/livez").to(livez))
        .service(web::resource("/readyz").to(readyz))
        .service(
            web::scope("/api")
                .configure(auth::auth_routes)
                .configure(webhook::webhook_routes)
                .service(
                    web::scope("/v1/servers")
                        .configure(server::server_routes)
                        .configure(channel::channel_routes),
                ),
        );
}
