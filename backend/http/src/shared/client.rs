use actix_web::{HttpRequest, http};
use application::dto::requests::client_context::ClientContext;

pub fn extract_client_context(req: &HttpRequest) -> ClientContext {
    let ip = req
        .connection_info()
        .realip_remote_addr()
        .map(|s| s.to_string());

    let user_agent = req
        .headers()
        .get(http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    ClientContext { ip, user_agent }
}
