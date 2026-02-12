use actix_web::web;

use crate::handlers::room::{create_room, generate_token};

pub fn room_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/room")
            .service(create_room)
            .service(generate_token),
    );
}
