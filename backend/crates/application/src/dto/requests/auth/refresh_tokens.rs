use crate::dto::requests::client_context::ClientContext;

#[derive(Debug)]
pub struct RefreshTokensRequest {
    pub refresh_token: String,
    pub client: ClientContext,
}
