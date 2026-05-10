use actix_web::web;

use crate::handlers::channel::{create_channel, delete_channel, join_channel, leave_channel};

pub fn channel_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/channel")
            .service(create_channel)
            .service(join_channel)
            .service(leave_channel)
            .service(delete_channel),
    );
}
