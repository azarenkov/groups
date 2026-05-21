use actix_web::web;

use crate::handlers::server::{
    create_server, create_server_invite, delete_server, list_server_channels,
    list_server_invites, list_server_members, list_servers_by_user, redeem_server_invite,
    revoke_server_invite,
};

pub fn server_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_server)
        .service(redeem_server_invite)
        .service(delete_server)
        .service(list_servers_by_user)
        .service(list_server_members)
        .service(list_server_channels)
        .service(create_server_invite)
        .service(list_server_invites)
        .service(revoke_server_invite);
}
