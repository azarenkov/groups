use crate::dto::RequestContext;

#[derive(Debug)]
pub struct CreateCommunityRequest {
    pub name: String,
    pub description: Option<String>,
    pub accent_color: Option<String>,
    pub ctx: RequestContext,
}
