use actix_web::{HttpMessage, HttpRequest};
use application::dto::RequestContext;
use shared::middleware::request_id::RequestId;

pub fn request_context(req: &HttpRequest) -> RequestContext {
    RequestContext {
        request_id: req.extensions().get::<RequestId>().map(|r| r.0.clone()),
    }
}
