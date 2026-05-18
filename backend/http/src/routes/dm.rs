use actix_web::web;

use crate::handlers::dm::{
    delete_dm_message, edit_dm_message, list_dm_messages, search_dm_messages,
    send_dm_message,
};

pub fn dm_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(search_dm_messages)
        .service(list_dm_messages)
        .service(send_dm_message)
        .service(edit_dm_message)
        .service(delete_dm_message);
}
