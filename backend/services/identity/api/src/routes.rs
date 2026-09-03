use actix_web::web;

use crate::auth::{
    confirm_registration, google_complete, google_login, login_user, logout, refresh_tokens,
    register_user, request_verification_code,
};
use crate::appearance::{get_me_appearance, update_me_appearance};
use crate::connections::{
    connection_callback, list_connections, remove_connection, start_connection, user_connections,
};
use crate::internal::internal_routes;
use crate::notification_settings::{
    get_me_notification_settings, update_me_notification_settings,
};
use crate::voice_settings::{get_me_voice_settings, update_me_voice_settings};
use crate::user::{
    change_me_email, confirm_me_email, delete_me, disable_me, get_me, search_users,
    update_me_locale, update_me_name, update_me_password, update_me_profile,
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
                        .service(delete_me)
                        .service(update_me_profile)
                        .service(get_me_appearance)
                        .service(update_me_appearance)
                        .service(get_me_voice_settings)
                        .service(update_me_voice_settings)
                        .service(get_me_notification_settings)
                        .service(update_me_notification_settings)
                        .service(list_connections)
                        .service(start_connection)
                        .service(remove_connection)
                        .service(user_connections),
                )
                .service(web::scope("/v1/connections").service(connection_callback)),
        );
}
