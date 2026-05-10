use actix_web::web;

use crate::handlers::channel::{
    create_channel, delete_channel, join_voice_channel, leave_voice_channel, mute_channel,
    unmute_channel,
};

pub fn channel_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/servers")
            .service(create_channel)
            .service(join_voice_channel)
            .service(leave_voice_channel)
            .service(mute_channel)
            .service(unmute_channel)
            .service(delete_channel),
    );
}
