use actix_web::web;

use crate::handlers::server::{
    create_server, delete_server, leave_server, list_sent_server_invites, list_server_channels,
    list_server_members, list_servers_by_user,
};

pub fn server_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_server)
        .service(delete_server)
        .service(leave_server)
        .service(list_servers_by_user)
        .service(list_server_members)
        .service(list_server_channels)
        .service(list_sent_server_invites);
}
