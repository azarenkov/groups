use actix_web::web;

use crate::handlers::friendship::{
    accept_friend_request, list_friends, remove_friendship, send_friend_request,
};

pub fn friendship_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(send_friend_request)
        .service(accept_friend_request)
        .service(remove_friendship)
        .service(list_friends);
}
