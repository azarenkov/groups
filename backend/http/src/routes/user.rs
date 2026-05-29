use actix_web::web;

use crate::handlers::user::{get_me, search_users, update_me_locale};

pub fn user_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_me);
    cfg.service(search_users);
    cfg.service(update_me_locale);
}
