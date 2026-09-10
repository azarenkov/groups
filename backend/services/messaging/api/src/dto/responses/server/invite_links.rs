use chrono::{DateTime, Utc};
use messaging::usecases::server::invite_links::{InviteJoin, InvitePreview};
use messaging_domain::server::invite_link::InviteLink;
use serde::Serialize;
use uuid::Uuid;

use crate::dto::responses::server::create_server::CreateServerResponse;

pub fn invite_url(public_app_url: &str, token: &str) -> String {
    format!("{}/join/{}", public_app_url.trim_end_matches('/'), token)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteLinkResponse {
    pub token: String,
    pub url: String,
    pub name: Option<String>,
    pub is_primary: bool,
    pub created_by: Uuid,
    pub creator_name: Option<String>,
    pub uses: i32,
    pub max_uses: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl InviteLinkResponse {
    pub fn from_link(link: InviteLink, public_app_url: &str) -> Self {
        Self {
            url: invite_url(public_app_url, link.token.as_str()),
            token: link.token.as_str().to_owned(),
            name: link.name,
            is_primary: link.is_primary,
            created_by: link.created_by.uuid(),
            creator_name: link.creator_name,
            uses: link.uses,
            max_uses: link.max_uses,
            expires_at: link.expires_at,
            created_at: link.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListInviteLinksResponse {
    pub links: Vec<InviteLinkResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitePreviewServerResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub accent_color: Option<String>,
    pub member_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitePreviewResponse {
    pub status: &'static str,
    pub server: Option<InvitePreviewServerResponse>,
}

impl From<InvitePreview> for InvitePreviewResponse {
    fn from(preview: InvitePreview) -> Self {
        Self {
            status: preview.status.as_str(),
            server: preview.server.map(|s| InvitePreviewServerResponse {
                id: s.id,
                name: s.name,
                description: s.description,
                accent_color: s.accent_color,
                member_count: s.member_count,
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinInviteResponse {
    pub server: CreateServerResponse,
    pub already_member: bool,
}

impl From<InviteJoin> for JoinInviteResponse {
    fn from(join: InviteJoin) -> Self {
        Self {
            server: CreateServerResponse::from(join.server),
            already_member: join.already_member,
        }
    }
}
