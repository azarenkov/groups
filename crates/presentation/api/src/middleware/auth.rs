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

const ACCESS_TOKEN_EXPIRATION_MINUTES: i64 = 15;
const REFRESH_TOKEN_EXPIRATION_DAYS: i64 = 120;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub id: Uuid,
    pub exp: usize,
    pub token_type: TokenType,
}

impl Claims {
    pub fn new(id: Uuid, exp: usize, token_type: TokenType) -> Self {
        Self {
            id,
            exp,
            token_type,
        }
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
            Ok(token) => {
                if token.claims.token_type != TokenType::Access {
                    return ready(Err(ErrorUnauthorized("Invalid token type")));
                }
                ready(Ok(AuthToken {
                    id: token.claims.id,
                }))
            }
            Err(_) => ready(Err(ErrorUnauthorized("Unauthorized"))),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

pub async fn encode_access_token(id: Uuid, secret: &str) -> anyhow::Result<String> {
    let exp =
        (Utc::now() + Duration::minutes(ACCESS_TOKEN_EXPIRATION_MINUTES)).timestamp() as usize;
    let claims = Claims::new(id, exp, TokenType::Access);
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    );
    token.context("Error encoding access token")
}

pub async fn encode_refresh_token(id: Uuid, secret: &str) -> anyhow::Result<String> {
    let exp = (Utc::now() + Duration::days(REFRESH_TOKEN_EXPIRATION_DAYS)).timestamp() as usize;
    let claims = Claims::new(id, exp, TokenType::Refresh);
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    );
    token.context("Error encoding refresh token")
}

pub async fn encode_token_pair(id: Uuid, secret: &str) -> anyhow::Result<TokenPair> {
    let access_token = encode_access_token(id, secret).await?;
    let refresh_token = encode_refresh_token(id, secret).await?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: ACCESS_TOKEN_EXPIRATION_MINUTES * 60,
    })
}

pub async fn validate_refresh_token(token: &str, secret: &str) -> anyhow::Result<Uuid> {
    let decode = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )?;

    if decode.claims.token_type != TokenType::Refresh {
        return Err(anyhow::anyhow!("Invalid token type"));
    }

    Ok(decode.claims.id)
}
