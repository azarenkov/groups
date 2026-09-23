use actix_web::web;

use crate::attachment::attachment_routes;
use crate::internal::internal_routes;
use crate::banner::{delete_my_banner, get_user_banner, upload_my_banner};
use crate::avatar::{
    delete_my_avatar, delete_community_logo, get_community_logo, get_user_avatar, upload_my_avatar,
    upload_community_logo,
};
use api_common::health::{livez, readyz};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/livez").to(livez))
        .service(web::resource("/readyz").to(readyz))
        .configure(internal_routes)
        .service(
            web::scope("/api")
                .configure(attachment_routes)
                .service(
                    web::scope("/v1/users")
                        .service(upload_my_avatar)
                        .service(delete_my_avatar)
                        .service(get_user_avatar)
                        .service(upload_my_banner)
                        .service(delete_my_banner)
                        .service(get_user_banner),
                )
                .service(
                    web::scope("/v1/communities")
                        .service(upload_community_logo)
                        .service(delete_community_logo)
                        .service(get_community_logo),
                ),
        );
}
