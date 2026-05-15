use actix_web::web;

use crate::handlers::channel::{
    create_channel, delete_channel, join_voice_channel, list_channel_participants, mute_channel,
    unmute_channel,
};

pub fn channel_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create_channel)
        .service(join_voice_channel)
        .service(mute_channel)
        .service(unmute_channel)
        .service(list_channel_participants)
        .service(delete_channel);
}
