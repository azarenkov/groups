use actix_web::{HttpRequest, http};
use application::dto::requests::client_context::ClientContext;
use shared::ip::real_ip_from_request;

pub fn extract_client_context(req: &HttpRequest) -> ClientContext {
    let ip = real_ip_from_request(req).map(|ip| ip.to_string());
    let user_agent = req
        .headers()
        .get(http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    ClientContext { ip, user_agent }
}
