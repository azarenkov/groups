use actix_web::{HttpRequest, http};
use application::dto::requests::client_context::ClientContext;

pub fn extract_client_context(req: &HttpRequest, trust_forwarded_headers: bool) -> ClientContext {
    let info = req.connection_info();
    let ip = if trust_forwarded_headers {
        info.realip_remote_addr().map(|s| s.to_string())
    } else {
        info.peer_addr().map(|s| s.to_string())
    };

    let user_agent = req
        .headers()
        .get(http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    ClientContext { ip, user_agent }
}
