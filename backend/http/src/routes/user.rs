use actix_web::web;

use crate::handlers::user::search_users;

pub fn user_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(search_users);
}
