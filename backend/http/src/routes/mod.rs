use actix_web::web;

pub mod auth;
pub mod channel;
pub mod server;
pub mod webhook;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(channel::channel_routes)
            .configure(auth::auth_routes)
            .configure(server::server_routes)
            .configure(webhook::webhook_routes),
    );
}
