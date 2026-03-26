use actix_web::web;

use crate::handlers::auth::{login_user, refresh_tokens, register_user};

pub fn auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/auth")
            .service(register_user)
            .service(login_user)
            .service(refresh_tokens),
    );
}
