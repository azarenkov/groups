use std::future::{Ready, ready};
use std::sync::LazyLock;

use actix_web::{Error as ActixWebError, http};
use actix_web::{FromRequest, HttpRequest, dev::Payload, error::ErrorUnauthorized};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::middleware::jwt::validation::validate_access_token;

static JWT_SECRET: LazyLock<String> =
    LazyLock::new(|| std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"));

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthToken {
    pub id: Uuid,
}

impl FromRequest for AuthToken {
    type Error = ActixWebError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let auth_header = match req.headers().get(http::header::AUTHORIZATION) {
            Some(header) => header,
            None => return ready(Err(ErrorUnauthorized("No authorization header"))),
        };

        let header_str = match auth_header.to_str() {
            Ok(s) => s,
            Err(_) => return ready(Err(ErrorUnauthorized("Invalid header encoding"))),
        };

        let auth_token = match header_str.strip_prefix("Bearer ") {
            Some(t) => t.trim(),
            None => return ready(Err(ErrorUnauthorized("Invalid auth scheme"))),
        };

        if auth_token.is_empty() {
            return ready(Err(ErrorUnauthorized("Invalid auth token")));
        }

        match validate_access_token(auth_token, JWT_SECRET.as_str()) {
            Ok(user_id) => ready(Ok(AuthToken { id: user_id })),
            Err(_) => ready(Err(ErrorUnauthorized("Unauthorized"))),
        }
    }
}
