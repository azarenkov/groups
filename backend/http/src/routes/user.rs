use actix_web::web;

use crate::handlers::avatar::{delete_my_avatar, get_user_avatar, upload_my_avatar};
use crate::handlers::user::{get_me, search_users, update_me_locale};

pub fn user_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_me);
    cfg.service(search_users);
    cfg.service(update_me_locale);
    cfg.service(upload_my_avatar);
    cfg.service(delete_my_avatar);
    cfg.service(get_user_avatar);
}
