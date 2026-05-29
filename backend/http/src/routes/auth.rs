use actix_web::web;

use crate::handlers::auth::{
    confirm_registration, google_login, login_user, logout, refresh_tokens, register_user,
    request_verification_code,
};

pub fn auth_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/auth")
            .service(register_user)
            .service(request_verification_code)
            .service(login_user)
            .service(google_login)
            .service(confirm_registration)
            .service(refresh_tokens)
            .service(logout),
    );
}
