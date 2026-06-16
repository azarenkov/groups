use std::future::{Ready, ready};
use std::sync::{Arc, LazyLock};

use actix_web::{
    Error as ActixWebError, FromRequest, HttpRequest, dev::Payload,
    error::{
        ErrorForbidden, ErrorInternalServerError, ErrorTooManyRequests, ErrorUnauthorized,
    },
    http, web,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{middleware::jwt::validation::validate_access_token, rate_limiting::UserRateLimiter};

static JWT_SECRET: LazyLock<String> =
    LazyLock::new(|| std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"));

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthToken {
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RateLimitedAuthToken {
    pub id: Uuid,
}

impl FromRequest for AuthToken {
    type Error = ActixWebError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(authenticate(req).map(|(id, _verified)| AuthToken { id }))
    }
}

impl FromRequest for RateLimitedAuthToken {
    type Error = ActixWebError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let (id, verified) = match authenticate(req) {
            Ok(v) => v,
            Err(e) => return ready(Err(e)),
        };
        if !verified {
            return ready(Err(ErrorForbidden("Email not verified")));
        }
        let limiter = match req.app_data::<web::Data<Arc<UserRateLimiter>>>() {
            Some(l) => l,
            None => {
                return ready(Err(ErrorInternalServerError(
                    "Rate limiter not configured",
                )));
            }
        };
        match limiter.check_key(&id) {
            Ok(_) => ready(Ok(RateLimitedAuthToken { id })),
            Err(_) => ready(Err(ErrorTooManyRequests("Too Many Requests"))),
        }
    }
}

fn authenticate(req: &HttpRequest) -> Result<(Uuid, bool), ActixWebError> {
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .ok_or_else(|| ErrorUnauthorized("No authorization header"))?;
    let header_str = auth_header
        .to_str()
        .map_err(|_| ErrorUnauthorized("Invalid header encoding"))?;
    let token = header_str
        .strip_prefix("Bearer ")
        .ok_or_else(|| ErrorUnauthorized("Invalid auth scheme"))?
        .trim();
    if token.is_empty() {
        return Err(ErrorUnauthorized("Invalid auth token"));
    }
    validate_access_token(token, JWT_SECRET.as_str())
        .map_err(|_| ErrorUnauthorized("Unauthorized"))
}
