pub mod activity_audience;
pub mod block_lookup;

pub(crate) fn with_request_id(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let builder = api_common::middleware::internal::with_internal_token(builder);
    match api_common::middleware::request_id::current() {
        Some(id) => builder.header(api_common::middleware::request_id::REQUEST_ID_HEADER, id),
        None => builder,
    }
}
