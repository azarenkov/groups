use crate::dto::RequestContext;

#[derive(Debug)]
pub struct CreateServerRequest {
    pub name: String,
    pub description: Option<String>,
    pub ctx: RequestContext,
}
