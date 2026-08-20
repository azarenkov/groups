use crate::dto::{RequestContext, requests::client_context::ClientContext};

#[derive(Debug)]
pub struct LoginUserRequest {
    pub email: String,
    pub password: String,
    pub client: ClientContext,
    pub ctx: RequestContext,
}
