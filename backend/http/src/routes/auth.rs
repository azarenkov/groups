use actix_web::web;

use crate::handlers::auth::{
    confirm_registration, login_user, logout, refresh_tokens, register_user,
};

pub fn auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/auth")
            .service(register_user)
            .service(login_user)
            .service(confirm_registration)
            .service(refresh_tokens)
            .service(logout),
    );
}
