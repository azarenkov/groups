use actix_web::web;

use crate::handlers::health::{livez, readyz};

pub mod auth;
pub mod channel;
pub mod dm;
pub mod friendship;
pub mod server;
pub mod user;
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
                )
                .service(
                    web::scope("/v1/users")
                        .configure(user::user_routes)
                        .configure(dm::dm_routes)
                        .configure(friendship::friendship_routes),
                )
                .service(
                    web::scope("/v1").configure(
                        crate::handlers::read_state::read_state_routes,
                    ),
                ),
        );
}
