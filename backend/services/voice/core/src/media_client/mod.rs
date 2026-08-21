pub mod friendship;

pub(crate) fn with_request_id(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match shared::middleware::request_id::current() {
        Some(id) => builder.header(shared::middleware::request_id::REQUEST_ID_HEADER, id),
        None => builder,
    }
}
