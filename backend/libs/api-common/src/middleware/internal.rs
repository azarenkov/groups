use std::sync::OnceLock;

use actix_web::body::{EitherBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{Error, HttpResponse};

pub const INTERNAL_TOKEN_HEADER: &str = "x-rift-internal-token";

const INTERNAL_PREFIX: &str = "/internal";

pub fn internal_token() -> Option<&'static str> {
    static TOKEN: OnceLock<Option<String>> = OnceLock::new();
    TOKEN
        .get_or_init(|| {
            let token = std::env::var("INTERNAL_API_TOKEN")
                .ok()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty());
            if token.is_none() {
                tracing::warn!("INTERNAL_API_TOKEN is not set; /internal routes accept any caller");
            }
            token
        })
        .as_deref()
}

pub fn with_internal_token(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match internal_token() {
        Some(token) => builder.header(INTERNAL_TOKEN_HEADER, token),
        None => builder,
    }
}

fn is_internal_path(path: &str) -> bool {
    path == INTERNAL_PREFIX || path.starts_with("/internal/")
}

fn token_matches(expected: &str, given: &[u8]) -> bool {
    let expected = expected.as_bytes();
    if expected.len() != given.len() {
        return false;
    }
    expected
        .iter()
        .zip(given)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

pub fn internal_request_allowed(path: &str, header: Option<&[u8]>, expected: Option<&str>) -> bool {
    if !is_internal_path(path) {
        return true;
    }
    match expected {
        None => true,
        Some(expected) => header.is_some_and(|given| token_matches(expected, given)),
    }
}

pub async fn guard_internal_routes(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<EitherBody<impl MessageBody>>, Error> {
    let header = req
        .headers()
        .get(INTERNAL_TOKEN_HEADER)
        .map(|value| value.as_bytes());
    if !internal_request_allowed(req.path(), header, internal_token()) {
        let response = HttpResponse::Forbidden().finish().map_into_right_body();
        return Ok(req.into_response(response));
    }
    next.call(req).await.map(ServiceResponse::map_into_left_body)
}

#[cfg(test)]
mod tests {
    use super::internal_request_allowed;

    #[test]
    fn public_routes_never_need_the_token() {
        assert!(internal_request_allowed("/api/v1/users/me", None, Some("secret")));
        assert!(internal_request_allowed("/internalize", None, Some("secret")));
    }

    #[test]
    fn internal_routes_need_the_exact_token() {
        assert!(internal_request_allowed("/internal/users/resolve", Some(b"secret"), Some("secret")));
        assert!(!internal_request_allowed("/internal/users/resolve", None, Some("secret")));
        assert!(!internal_request_allowed("/internal/users/resolve", Some(b"secreT"), Some("secret")));
        assert!(!internal_request_allowed("/internal", Some(b"secre"), Some("secret")));
    }

    #[test]
    fn an_unconfigured_token_keeps_internal_routes_open() {
        assert!(internal_request_allowed("/internal/entitlements", None, None));
    }
}
