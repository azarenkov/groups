pub mod claims;
pub mod identity_users;
pub mod voice_calls;

pub(crate) fn with_request_id(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match api_common::middleware::request_id::current() {
        Some(id) => builder.header(api_common::middleware::request_id::REQUEST_ID_HEADER, id),
        None => builder,
    }
}
