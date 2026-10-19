use domain::user::role::UserRole;

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct CreateCommunityRequest {
    pub name: String,
    pub description: Option<String>,
    pub accent_color: Option<String>,
    pub creator_role: UserRole,
    pub ctx: RequestContext,
}
