use actix_web::web;

use crate::handlers::room::{create_room, delete_room, join_room};

pub fn room_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/room")
            .service(create_room)
            .service(join_room)
            .service(delete_room),
    );
}
