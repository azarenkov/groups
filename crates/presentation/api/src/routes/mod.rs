use actix_web::web;

pub mod auth;
pub mod room;
pub mod server;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(room::room_routes)
            .configure(auth::auth_routes)
            .configure(server::server_routes),
    );
}
