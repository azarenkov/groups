use actix_web::web;

pub mod room;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api").configure(room::room_routes));
}
