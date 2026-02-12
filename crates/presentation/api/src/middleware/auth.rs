use std::future::{Ready, ready};

use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::{Error as ActixWebError, web};
use actix_web::{
    FromRequest,
    http::{self},
};
use anyhow::Context;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::state::AppState;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub id: Uuid,
    pub exp: usize,
}

impl Claims {
    fn new(id: Uuid, exp: usize) -> Self {
        Self { id, exp }
    }
}

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

        let decode = decode::<Claims>(
            auth_token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::new(jsonwebtoken::Algorithm::HS256),
        );

        match decode {
            Ok(token) => ready(Ok(AuthToken {
                id: token.claims.id,
            })),
            Err(_) => ready(Err(ErrorUnauthorized("Unauthorized"))),
        }
    }
}

pub async fn encode_token(id: Uuid, secret: &str) -> anyhow::Result<String> {
    let exp = (Utc::now() + Duration::days(365)).timestamp() as usize;
    let claims = Claims::new(id, exp);
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    );
    token.context("Errod encode token()")
}
