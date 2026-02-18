use std::future::{Ready, ready};

use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::{Error as ActixWebError, web};
use actix_web::{
    FromRequest,
    http::{self},
};
use serde::{Deserialize, Serialize};
use shared::jwt::validation::validate_access_token;
use uuid::Uuid;

use crate::shared::state::AppState;

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthToken {
    pub id: Uuid,
}

impl FromRequest for AuthToken {
    type Error = ActixWebError;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let auth_header = match req.headers().get(http::header::AUTHORIZATION) {
            Some(header) => header,
            None => return ready(Err(ErrorUnauthorized("No authorization header"))),
        };

        let auth_token = auth_header
            .to_str()
            .unwrap_or("")
            .trim_start_matches("Bearer ")
            .trim();
        if auth_token.is_empty() {
            return ready(Err(ErrorUnauthorized("Invalid auth token")));
        }

        let secret = match req.app_data::<web::Data<AppState>>() {
            Some(app_state) => app_state.jwt_secret(),
            None => return ready(Err(ErrorInternalServerError("App state not found"))),
        };

        match validate_access_token(auth_token, secret) {
            Ok(user_id) => ready(Ok(AuthToken { id: user_id })),
            Err(_) => ready(Err(ErrorUnauthorized("Unauthorized"))),
        }
    }
}
