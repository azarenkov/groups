use actix_web::web;

use crate::handlers::avatar::{delete_server_logo, get_server_logo, upload_server_logo};
use crate::handlers::server::{
    create_invite_code, create_server, delete_server, leave_server, list_invite_codes,
    list_sent_server_invites, list_server_channels, list_server_members, list_servers_by_user,
    list_voice_presence, redeem_invite_code, revoke_invite_code, update_server,
    update_stream_viewer,
};

pub fn server_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_server)
        .service(redeem_invite_code)
        .service(create_invite_code)
        .service(list_invite_codes)
        .service(revoke_invite_code)
        .service(update_server)
        .service(delete_server)
        .service(leave_server)
        .service(list_servers_by_user)
        .service(list_server_members)
        .service(list_server_channels)
        .service(list_voice_presence)
        .service(update_stream_viewer)
        .service(list_sent_server_invites)
        .service(upload_server_logo)
        .service(delete_server_logo)
        .service(get_server_logo);
}
