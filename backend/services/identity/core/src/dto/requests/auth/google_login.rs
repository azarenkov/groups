use identity_domain::user::locale::Locale;

use crate::dto::{RequestContext, requests::client_context::ClientContext};

#[derive(Debug)]
pub struct GoogleLoginRequest {
    pub id_token: String,
    pub locale: Locale,
    pub client: ClientContext,
    pub ctx: RequestContext,
}
