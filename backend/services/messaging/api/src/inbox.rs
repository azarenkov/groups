use actix_web::{HttpRequest, HttpResponse, Responder, get, post, web};
use api_common::{context::request_context, dto::responses::ApiResponse, middleware::auth::RateLimitedAuthToken};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging::usecases::inbox::read_inbox::InboxPage;
use messaging_domain::{
    community::{id::CommunityId, invitation::{Invitation, InvitationId}},
    inbox::{InboxItem, InboxItemKind},
};
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueInvitationBody {
    user_id: Uuid,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InvitationResponse {
    id: Uuid,
    community_id: Uuid,
    inviter_id: Uuid,
    invitee_id: Uuid,
    created_at: DateTime<Utc>,
}

impl From<Invitation> for InvitationResponse {
    fn from(invitation: Invitation) -> Self {
        Self {
            id: invitation.id().uuid(),
            community_id: invitation.community_id().uuid(),
            inviter_id: invitation.inviter_id().uuid(),
            invitee_id: invitation.invitee_id().uuid(),
            created_at: invitation.created_at(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AcceptedInvitationResponse {
    community_id: Uuid,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InboxActorResponse {
    id: Uuid,
    name: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InboxCommunityResponse {
    id: Uuid,
    name: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InboxItemResponse {
    id: Uuid,
    kind: &'static str,
    actor: InboxActorResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    community: Option<InboxCommunityResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invitation_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    read: bool,
}

impl From<InboxItem> for InboxItemResponse {
    fn from(item: InboxItem) -> Self {
        let (kind, community, invitation_id) = match item.kind {
            InboxItemKind::CommunityInvitation { invitation_id, community_id, community_name } => (
                "community_invitation",
                Some(InboxCommunityResponse { id: community_id.uuid(), name: community_name }),
                Some(invitation_id.uuid()),
            ),
            InboxItemKind::RemovedFromCommunity { community_id, community_name } => (
                "removed_from_community",
                Some(InboxCommunityResponse { id: community_id.uuid(), name: community_name }),
                None,
            ),
            InboxItemKind::FriendRequestReceived => ("friend_request_received", None, None),
            InboxItemKind::FriendRequestAccepted => ("friend_request_accepted", None, None),
        };
        Self {
            id: item.id,
            kind,
            actor: InboxActorResponse { id: item.actor_id.uuid(), name: item.actor_name },
            community,
            invitation_id,
            created_at: item.created_at,
            read: item.read,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InboxPageResponse {
    items: Vec<InboxItemResponse>,
    next_cursor: Option<String>,
}

impl From<InboxPage> for InboxPageResponse {
    fn from(page: InboxPage) -> Self {
        Self { items: page.items.into_iter().map(Into::into).collect(), next_cursor: page.next_cursor }
    }
}

#[derive(Debug, serde::Deserialize)]
struct InboxQuery {
    before: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
struct UnreadResponse {
    count: i64,
}

#[derive(Debug, serde::Deserialize)]
struct MarkReadBody {
    ids: Option<Vec<Uuid>>,
}

#[post("/{community_id}/invitations")]
pub async fn issue_invitation(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<IssueInvitationBody>,
) -> Result<impl Responder, HttpError> {
    let invitation = app_state
        .send_community_invite_usecase
        .execute(
            CommunityId::new(path.into_inner()),
            UserId::new(body.user_id),
            UserId::new(auth_token.id),
            request_context(&req),
        )
        .await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(InvitationResponse::from(invitation))))
}

#[post("/invitations/{invitation_id}/accept")]
async fn accept_invitation(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let community_id = app_state
        .accept_community_invite_usecase
        .execute(InvitationId::new(path.into_inner()), UserId::new(auth_token.id), request_context(&req))
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(AcceptedInvitationResponse { community_id })))
}

#[post("/invitations/{invitation_id}/decline")]
async fn decline_invitation(
    req: HttpRequest,
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    app_state
        .decline_community_invite_usecase
        .execute(InvitationId::new(path.into_inner()), UserId::new(auth_token.id), request_context(&req))
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/inbox")]
async fn list_inbox(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    query: web::Query<InboxQuery>,
) -> Result<impl Responder, HttpError> {
    let page = app_state
        .read_inbox_usecase
        .list(UserId::new(auth_token.id), query.before.as_deref(), query.limit)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(InboxPageResponse::from(page))))
}

#[get("/inbox/unread")]
async fn inbox_unread(auth_token: RateLimitedAuthToken, app_state: web::Data<AppState>) -> Result<impl Responder, HttpError> {
    let count = app_state.read_inbox_usecase.unread_count(UserId::new(auth_token.id)).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(UnreadResponse { count })))
}

#[post("/inbox/read")]
async fn mark_inbox_read(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<MarkReadBody>,
) -> Result<impl Responder, HttpError> {
    let ids = body.into_inner().ids;
    app_state.read_inbox_usecase.mark_read(UserId::new(auth_token.id), ids).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn inbox_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_inbox)
        .service(inbox_unread)
        .service(mark_inbox_read)
        .service(accept_invitation)
        .service(decline_invitation);
}

#[cfg(test)]
mod tests {
    use actix_web::{App, http::StatusCode, test};

    use super::*;

    async fn echo(body: web::Json<MarkReadBody>) -> HttpResponse {
        HttpResponse::Ok().body(body.into_inner().ids.map_or("all".to_string(), |ids| ids.len().to_string()))
    }

    #[actix_web::test]
    async fn mark_read_bodies_must_be_valid_json() {
        let app = test::init_service(App::new().route("/read", web::post().to(echo))).await;
        let all = test::TestRequest::post().uri("/read").set_json(serde_json::json!({})).to_request();
        assert_eq!(test::call_and_read_body(&app, all).await, "all");
        let some = test::TestRequest::post().uri("/read").set_json(serde_json::json!({ "ids": [Uuid::new_v4()] })).to_request();
        assert_eq!(test::call_and_read_body(&app, some).await, "1");
        let truncated = test::TestRequest::post()
            .uri("/read")
            .insert_header(("content-type", "application/json"))
            .set_payload("{\"ids\": [")
            .to_request();
        assert_eq!(test::call_service(&app, truncated).await.status(), StatusCode::BAD_REQUEST);
        let wrong_type = test::TestRequest::post().uri("/read").insert_header(("content-type", "text/plain")).set_payload("{}").to_request();
        assert_eq!(test::call_service(&app, wrong_type).await.status(), StatusCode::BAD_REQUEST);
    }
}
