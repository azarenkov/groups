use actix_web::web;

use crate::attachments::attachment_routes;
use crate::badges::badge_routes;
use crate::blocks::block_routes;
use crate::calls::call_log_routes;
use crate::channel::{
    create_channel, delete_channel, delete_channel_message, edit_channel_message,
    join_text_channel, join_voice_channel, list_channel_messages, list_channel_participants,
    deafen_channel, disconnect_voice_member, get_channel_message, hide_channel_message, mute_channel, search_channel_messages, send_channel_message, undeafen_channel, unmute_channel,
};
use crate::dm::{
    delete_dm_message, edit_dm_message,
    get_dm_message, hide_dm_message, list_dm_messages, list_dm_previews, search_dm_messages, send_dm_message,
    };
use crate::home::{get_home_overview, get_user_profile};
use crate::internal::internal_routes;
use crate::friendship::{
    accept_friend_request, list_friend_ids, list_friends, remove_friendship, send_friend_request,
};
use crate::notifications::notification_routes;
use crate::presence::presence_routes;
use crate::read_state::read_state_routes;
use crate::reports::{community_report_routes, sticker_pack_report_routes, user_report_routes};
use crate::roles::role_routes;
use crate::invites::invite_routes;
use crate::server::{
    create_invite_link, create_community, delete_community, kick_member, leave_community, list_invite_links,
    list_sent_community_invites, list_community_channels, list_community_members, list_communities_by_user,
    list_voice_presence, revoke_invite_link, update_community,
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
                .configure(sticker_pack_report_routes)
                .configure(attachment_routes)
                .configure(invite_routes)
                .service(
                    web::scope("/v1/communities")
                        .configure(role_routes)
                        .configure(community_report_routes)
                        .service(create_community)
                        .service(create_invite_link)
                        .service(list_invite_links)
                        .service(revoke_invite_link)
                        .service(update_community)
                        .service(delete_community)
                        .service(leave_community)
                        .service(kick_member)
                        .service(list_communities_by_user)
                        .service(list_community_members)
                        .service(list_community_channels)
                        .service(list_voice_presence)
                        .service(update_stream_viewer)
                        .service(list_sent_community_invites)
                        .service(crate::inbox::issue_invitation)
                        .service(create_channel)
                        .service(join_voice_channel)
                        .service(join_text_channel)
                        .service(search_channel_messages)
                        .configure(crate::shared::channel_shared_routes)
                        .configure(crate::pins::channel_pin_routes)
                        .service(get_channel_message)
                        .service(disconnect_voice_member)
                        .service(hide_channel_message)
                        .service(crate::voice::mark_channel_voice_listened)
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
                        .configure(crate::pins::user_pin_routes)
                        .configure(crate::dm_conversations::dm_conversation_routes)
                        .configure(badge_routes)
                        .service(crate::forward::forward_message)
                        .service(search_dm_messages)
                        .configure(crate::shared::dm_shared_routes)
                        .service(get_dm_message)
                        .service(hide_dm_message)
                        .service(crate::voice::mark_dm_voice_listened)
                        .service(list_dm_previews)
                        .service(list_dm_messages)
                        .service(send_dm_message)
                        .service(edit_dm_message)
                        .service(delete_dm_message)
                        .service(list_friend_ids)
                        .service(list_friends)
                        .service(send_friend_request)
                        .service(accept_friend_request)
                        .service(remove_friendship)
                        .service(get_home_overview)
                        .service(get_user_profile)
                        .configure(crate::privacy::privacy_routes)
                        .configure(user_report_routes)
                        .configure(block_routes),
                )
                .service(
                    web::scope("/v1")
                        .configure(read_state_routes)
                        .configure(notification_routes)
                        .configure(presence_routes)
                        .configure(crate::search::search_routes)
                        .configure(crate::inbox::inbox_routes),
                ),
        );
}
