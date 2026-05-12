use std::{net::IpAddr, sync::LazyLock};

use actix_governor::{KeyExtractor, SimpleKeyExtractionError};
use actix_web::{dev::ServiceRequest, http};
use uuid::Uuid;

use crate::{
    ip::real_ip_from_service_request, middleware::jwt::validation::validate_access_token,
};

static JWT_SECRET: LazyLock<String> =
    LazyLock::new(|| std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"));

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitKey {
    User(Uuid),
    Ip(IpAddr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserOrIpKeyExtractor;

impl KeyExtractor for UserOrIpKeyExtractor {
    type Key = RateLimitKey;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        if let Some(user_id) = extract_user_id(req) {
            return Ok(RateLimitKey::User(user_id));
        }
        real_ip_from_service_request(req)
            .map(RateLimitKey::Ip)
            .ok_or_else(|| SimpleKeyExtractionError::new("Could not extract rate limit key"))
    }
}

fn extract_user_id(req: &ServiceRequest) -> Option<Uuid> {
    let header = req.headers().get(http::header::AUTHORIZATION)?;
    let header_str = header.to_str().ok()?;
    let token = header_str.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        return None;
    }
    validate_access_token(token, JWT_SECRET.as_str()).ok()
}
