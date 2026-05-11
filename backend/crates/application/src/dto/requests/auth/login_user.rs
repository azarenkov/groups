use crate::dto::requests::client_context::ClientContext;

#[derive(Debug)]
pub struct LoginUserRequest {
    pub email: String,
    pub password: String,
    pub client: ClientContext,
}
