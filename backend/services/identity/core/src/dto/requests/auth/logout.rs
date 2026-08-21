use crate::dto::{RequestContext, requests::client_context::ClientContext};

#[derive(Debug)]
pub struct LogoutRequest {
    pub refresh_token: String,
    pub client: ClientContext,
    pub ctx: RequestContext,
}
