use std::sync::LazyLock;

use actix_web::{
    Error as ActixWebError, FromRequest, HttpRequest, dev::Payload,
    error::{
        ErrorForbidden, ErrorInternalServerError, ErrorTooManyRequests, ErrorUnauthorized,
    },
    http, web,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    middleware::jwt::validation::{ValidatedAccess, validate_access_token},
    middleware::revocation::AccessRevocations,
    rate_limiting::KeyedRateLimiter,
};

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
    type Future = futures::future::LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let auth_result = authenticate(req);
        let revocations = req.app_data::<web::Data<AccessRevocations>>().cloned();
        Box::pin(async move {
            let access = auth_result?;
            ensure_not_revoked(revocations.as_ref().map(|d| d.get_ref()), &access).await?;
            Ok(AuthToken { id: access.user_id })
        })
    }
}

async fn ensure_not_revoked(
    revocations: Option<&AccessRevocations>,
    access: &ValidatedAccess,
) -> Result<(), ActixWebError> {
    if let Some(store) = revocations
        && store.is_revoked(&access.user_id, access.issued_at).await
    {
        return Err(ErrorUnauthorized("Unauthorized"));
    }
    Ok(())
}

impl FromRequest for RateLimitedAuthToken {
    type Error = ActixWebError;
    type Future = futures::future::LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let auth_result = authenticate(req);
        let limiter = req.app_data::<web::Data<KeyedRateLimiter>>().cloned();
        let revocations = req.app_data::<web::Data<AccessRevocations>>().cloned();
        Box::pin(async move {
            let access = auth_result?;
            ensure_not_revoked(revocations.as_ref().map(|d| d.get_ref()), &access).await?;
            let ValidatedAccess { user_id: id, verified, .. } = access;
            if !verified {
                return Err(ErrorForbidden("Email not verified"));
            }
            let limiter = limiter
                .ok_or_else(|| ErrorInternalServerError("Rate limiter not configured"))?;
            if limiter.check_key(&id).await {
                Ok(RateLimitedAuthToken { id })
            } else {
                Err(ErrorTooManyRequests("Too Many Requests"))
            }
        })
    }
}

fn authenticate(req: &HttpRequest) -> Result<ValidatedAccess, ActixWebError> {
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
