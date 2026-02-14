use actix_web::web;

use crate::handlers::server::{create_server, delete_server};

pub fn server_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/server")
            .service(create_server)
            .service(delete_server),
    );
}
