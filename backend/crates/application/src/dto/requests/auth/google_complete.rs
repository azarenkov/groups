use domain::user::locale::Locale;

use crate::dto::{RequestContext, requests::client_context::ClientContext};

#[derive(Debug)]
pub struct GoogleCompleteRequest {
    pub id_token: String,
    pub username: String,
    pub locale: Locale,
    pub client: ClientContext,
    pub ctx: RequestContext,
}
