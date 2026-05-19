pub mod auth;
pub mod channel;
pub mod dm;
pub mod friendship;
pub mod server;

use serde::Serialize;

use crate::shared::dto::pagination::PaginationMeta;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationMeta>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { data, meta: None }
    }

    pub fn with_pagination(data: T, pagination: PaginationMeta) -> Self {
        Self {
            data,
            meta: Some(ResponseMeta {
                pagination: Some(pagination),
            }),
        }
    }
}
