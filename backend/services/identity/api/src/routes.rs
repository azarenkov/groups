use actix_web::web;

use crate::auth::{
    captcha_challenge, confirm_registration, forgot_password, google_complete, google_login, login_user, logout,
    refresh_tokens, register_user, request_verification_code, reset_password,
};
use crate::activity::{clear_me_activity, list_activities, set_me_activity};
use crate::appearance::{get_me_appearance, update_me_appearance};
use crate::connections::{
    connection_callback, list_connections, remove_connection, start_connection, user_connections,
};
use crate::internal::internal_routes;
use crate::hotkey_settings::{get_me_hotkey_settings, update_me_hotkey_settings};
use crate::notification_settings::{
    get_me_notification_settings, update_me_notification_settings,
};
use crate::voice_settings::{get_me_voice_settings, update_me_voice_settings};
use crate::user::{
    change_me_email, confirm_me_email, delete_me, disable_me, get_me, search_users,
    update_me_locale, update_me_name, update_me_password, update_me_profile,
};
use crate::entitlements::{entitlement_routes, internal_entitlement_routes};
use api_common::health::{livez, readyz};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/livez").to(livez))
        .service(web::resource("/readyz").to(readyz))
        .configure(internal_routes)
        .configure(internal_entitlement_routes)
        .service(
            web::scope("/api")
                .service(
                    web::scope("/v1/auth")
                        .service(captcha_challenge)
                        .service(register_user)
                        .service(request_verification_code)
                        .service(login_user)
                        .service(forgot_password)
                        .service(reset_password)
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
                        .configure(entitlement_routes)
                        .service(get_me_appearance)
                        .service(update_me_appearance)
                        .service(set_me_activity)
                        .service(clear_me_activity)
                        .service(list_activities)
                        .service(get_me_voice_settings)
                        .service(update_me_voice_settings)
                        .service(get_me_notification_settings)
                        .service(update_me_notification_settings)
                        .service(get_me_hotkey_settings)
                        .service(update_me_hotkey_settings)
                        .service(list_connections)
                        .service(start_connection)
                        .service(remove_connection)
                        .service(user_connections),
                )
                .service(web::scope("/v1/connections").service(connection_callback)),
        );
}
