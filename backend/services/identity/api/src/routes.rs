use actix_web::web;

use crate::auth::{
    confirm_registration, google_complete, google_login, login_user, logout, refresh_tokens,
    register_user, request_verification_code,
};
use crate::internal::internal_routes;
use crate::user::{
    change_me_email, confirm_me_email, delete_me, disable_me, get_me, search_users,
    update_me_locale, update_me_name, update_me_password,
};
use api_common::health::{livez, readyz};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/livez").to(livez))
        .service(web::resource("/readyz").to(readyz))
        .configure(internal_routes)
        .service(
            web::scope("/api")
                .service(
                    web::scope("/v1/auth")
                        .service(register_user)
                        .service(request_verification_code)
                        .service(login_user)
                        .service(google_login)
                        .service(google_complete)
                        .service(confirm_registration)
                        .service(refresh_tokens)
                        .service(logout),
                )
                .service(
                    web::scope("/v1/users")
                        .service(get_me)
                        .service(search_users)
                        .service(update_me_locale)
                        .service(update_me_name)
                        .service(update_me_password)
                        .service(change_me_email)
                        .service(confirm_me_email)
                        .service(disable_me)
                        .service(delete_me),
                ),
        );
}
