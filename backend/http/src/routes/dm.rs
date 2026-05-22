use actix_web::web;

use crate::handlers::dm::{
    accept_server_invite, delete_dm_message, edit_dm_message, list_dm_messages,
    search_dm_messages, send_dm_message, send_server_invite,
};

pub fn dm_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(search_dm_messages)
        .service(list_dm_messages)
        .service(send_dm_message)
        .service(send_server_invite)
        .service(accept_server_invite)
        .service(edit_dm_message)
        .service(delete_dm_message);
}
