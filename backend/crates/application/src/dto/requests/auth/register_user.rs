use crate::dto::requests::client_context::ClientContext;

#[derive(Debug)]
pub struct RegisterUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub client: ClientContext,
}
