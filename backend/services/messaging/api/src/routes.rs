use actix_web::web;

use crate::attachments::attachment_routes;
use crate::calls::call_log_routes;
use crate::channel::{
    create_channel, delete_channel, delete_channel_message, edit_channel_message,
    join_text_channel, join_voice_channel, list_channel_messages, list_channel_participants,
    deafen_channel, mute_channel, search_channel_messages, send_channel_message, undeafen_channel, unmute_channel,
};
use crate::dm::{
    accept_server_invite, decline_server_invite, delete_dm_message, edit_dm_message,
    list_dm_messages, list_dm_previews, search_dm_messages, send_dm_message,
    send_server_invite,
};
use crate::home::{get_home_overview, get_user_profile};
use crate::internal::internal_routes;
use crate::friendship::{
    accept_friend_request, list_friend_ids, list_friends, remove_friendship, send_friend_request,
};
use crate::notifications::notification_routes;
use crate::presence::presence_routes;
use crate::read_state::read_state_routes;
use crate::roles::role_routes;
use crate::invites::invite_routes;
use crate::server::{
    create_invite_link, create_server, delete_server, kick_member, leave_server, list_invite_links,
    list_sent_server_invites, list_server_channels, list_server_members, list_servers_by_user,
    list_voice_presence, revoke_invite_link, update_server,
    update_stream_viewer,
};
use api_common::health::{livez, readyz};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/livez").to(livez))
        .service(web::resource("/readyz").to(readyz))
        .configure(internal_routes)
        .service(
            web::scope("/api")
                .configure(call_log_routes)
                .configure(attachment_routes)
                .configure(invite_routes)
                .service(
                    web::scope("/v1/servers")
                        .configure(role_routes)
                        .service(create_server)
                        .service(create_invite_link)
                        .service(list_invite_links)
                        .service(revoke_invite_link)
                        .service(update_server)
                        .service(delete_server)
                        .service(leave_server)
                        .service(kick_member)
                        .service(list_servers_by_user)
                        .service(list_server_members)
                        .service(list_server_channels)
                        .service(list_voice_presence)
                        .service(update_stream_viewer)
                        .service(list_sent_server_invites)
                        .service(create_channel)
                        .service(join_voice_channel)
                        .service(join_text_channel)
                        .service(search_channel_messages)
                        .service(list_channel_messages)
                        .service(send_channel_message)
                        .service(edit_channel_message)
                        .service(delete_channel_message)
                        .service(mute_channel)
                        .service(unmute_channel)
                        .service(deafen_channel)
                        .service(undeafen_channel)
                        .service(list_channel_participants)
                        .service(delete_channel),
                )
                .service(
                    web::scope("/v1/users")
                        .service(search_dm_messages)
                        .service(list_dm_previews)
                        .service(list_dm_messages)
                        .service(send_dm_message)
                        .service(send_server_invite)
                        .service(accept_server_invite)
                        .service(decline_server_invite)
                        .service(edit_dm_message)
                        .service(delete_dm_message)
                        .service(send_friend_request)
                        .service(accept_friend_request)
                        .service(remove_friendship)
                        .service(list_friend_ids)
                        .service(list_friends)
                        .service(get_home_overview)
                        .service(get_user_profile)
                        .configure(crate::privacy::privacy_routes),
                )
                .service(
                    web::scope("/v1")
                        .configure(read_state_routes)
                        .configure(notification_routes)
                        .configure(presence_routes),
                ),
        );
}
