pub mod activity_audience;
pub mod block_lookup;
pub mod user_state;

pub(crate) fn with_request_id(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match api_common::middleware::request_id::current() {
        Some(id) => builder.header(api_common::middleware::request_id::REQUEST_ID_HEADER, id),
        None => builder,
    }
}
