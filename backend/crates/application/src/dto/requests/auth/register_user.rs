use domain::user::locale::Locale;

use crate::dto::{RequestContext, requests::client_context::ClientContext};

#[derive(Debug)]
pub struct RegisterUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub locale: Locale,
    pub client: ClientContext,
    pub ctx: RequestContext,
}
