use domain::user::role::UserRole;
use identity_domain::user::locale::Locale;

use crate::dto::{RequestContext, requests::client_context::ClientContext};

#[derive(Debug)]
pub struct RegisterUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub locale: Locale,
    pub role: UserRole,
    pub client: ClientContext,
    pub ctx: RequestContext,
}
