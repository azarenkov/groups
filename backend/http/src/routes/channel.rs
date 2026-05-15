use actix_web::web;

use crate::handlers::channel::{
    create_channel, delete_channel, delete_channel_message, edit_channel_message,
    join_text_channel, join_voice_channel, list_channel_participants, mute_channel,
    send_channel_message, unmute_channel,
};

pub fn channel_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_channel)
        .service(join_voice_channel)
        .service(join_text_channel)
        .service(send_channel_message)
        .service(edit_channel_message)
        .service(delete_channel_message)
        .service(mute_channel)
        .service(unmute_channel)
        .service(list_channel_participants)
        .service(delete_channel);
}
