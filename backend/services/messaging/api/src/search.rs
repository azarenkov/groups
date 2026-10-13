use actix_web::{HttpResponse, Responder, delete, get, post, web};
use api_common::{
    dto::responses::{ApiResponse, attachment::AttachmentResponse},
    middleware::auth::RateLimitedAuthToken,
};
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging::usecases::search::global_search::{GlobalSearchPage, GlobalSearchQuery};
use messaging_domain::people::PersonRelation;
use messaging_domain::search_history::{HistoryItem, HistoryTarget};
use messaging_domain::search::{SearchHit, SearchItemKey, SearchMatch, SearchPlace};
use uuid::Uuid;

use crate::{http_errors::HttpError, state::AppState};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchParams {
    q: String,
    kind: Option<String>,
    before: Option<String>,
    limit: Option<i64>,
}

impl From<SearchParams> for GlobalSearchQuery {
    fn from(params: SearchParams) -> Self {
        Self {
            query: params.q,
            kind: params.kind,
            before: params.before,
            limit: params.limit,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SearchPlaceResponse {
    #[serde(rename_all = "camelCase")]
    Channel {
        community_id: Uuid,
        community_name: String,
        channel_id: Uuid,
        channel_name: String,
    },
    #[serde(rename_all = "camelCase")]
    Dm { peer_id: Uuid, peer_name: String },
}

impl From<SearchPlace> for SearchPlaceResponse {
    fn from(place: SearchPlace) -> Self {
        match place {
            SearchPlace::Channel {
                community_id,
                community_name,
                channel_id,
                channel_name,
            } => Self::Channel {
                community_id,
                community_name,
                channel_id,
                channel_name,
            },
            SearchPlace::Dm { peer_id, peer_name } => Self::Dm { peer_id, peer_name },
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchHitResponse {
    message_id: Uuid,
    author_id: Uuid,
    author_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    forwarded_author_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    place: SearchPlaceResponse,
    snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachment: Option<AttachmentResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    position: Option<i64>,
}

impl From<SearchHit> for SearchHitResponse {
    fn from(hit: SearchHit) -> Self {
        let position = match hit.cursor.item {
            SearchItemKey::Link(ordinal) => Some(ordinal),
            SearchItemKey::Message | SearchItemKey::Attachment(_) => None,
        };
        let (attachment, url) = match hit.found {
            SearchMatch::Message => (None, None),
            SearchMatch::Attachment(link) => (Some(AttachmentResponse::from(link)), None),
            SearchMatch::Link(url) => (None, Some(url)),
        };
        Self {
            message_id: hit.message_id,
            author_id: hit.author_id.uuid(),
            author_name: hit.author_name,
            forwarded_author_id: hit.forwarded_author_id.map(|id| id.uuid()),
            created_at: hit.created_at,
            place: hit.place.into(),
            snippet: hit.snippet,
            attachment,
            url,
            position,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchPageResponse {
    items: Vec<SearchHitResponse>,
    next_cursor: Option<String>,
}

impl From<GlobalSearchPage> for SearchPageResponse {
    fn from(page: GlobalSearchPage) -> Self {
        Self {
            items: page.items.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        }
    }
}

#[get("/search")]
async fn global_search(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    params: web::Query<SearchParams>,
) -> Result<impl Responder, HttpError> {
    let page = app_state
        .global_search_usecase
        .execute(UserId::new(auth_token.id), params.into_inner().into())
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(SearchPageResponse::from(page))))
}

#[derive(Debug, serde::Deserialize)]
struct PeopleParams {
    q: String,
    limit: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum PersonRelationResponse {
    None,
    Friend,
    Outgoing,
    Incoming,
}

impl From<PersonRelation> for PersonRelationResponse {
    fn from(relation: PersonRelation) -> Self {
        match relation {
            PersonRelation::None => Self::None,
            PersonRelation::Friend => Self::Friend,
            PersonRelation::OutgoingRequest => Self::Outgoing,
            PersonRelation::IncomingRequest => Self::Incoming,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PersonResponse {
    id: Uuid,
    name: String,
    relation: PersonRelationResponse,
    accepts_friend_requests: bool,
}

#[derive(Debug, serde::Serialize)]
struct PeopleResponse {
    users: Vec<PersonResponse>,
}

#[get("/search/people")]
async fn search_people(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    params: web::Query<PeopleParams>,
) -> Result<impl Responder, HttpError> {
    if !app_state.people_search_limiter.check_key(&auth_token.id).await {
        return Err(HttpError::new(
            "Too many people searches. Slow down.".to_string(),
            actix_web::http::StatusCode::TOO_MANY_REQUESTS,
            "PEOPLE_SEARCH_RATE_LIMITED",
        ));
    }
    let found = app_state
        .search_people_usecase
        .execute(UserId::new(auth_token.id), &params.q, params.limit)
        .await?;
    let users = found
        .into_iter()
        .map(|person| PersonResponse {
            id: person.user_id.uuid(),
            name: person.name,
            relation: person.relation.into(),
            accepts_friend_requests: person.accepts_friend_requests,
        })
        .collect();
    Ok(HttpResponse::Ok().json(ApiResponse::success(PeopleResponse { users })))
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum HistoryTargetResponse {
    Query {
        text: String,
    },
    #[serde(rename_all = "camelCase")]
    Person { user_id: Uuid, name: String },
    #[serde(rename_all = "camelCase")]
    Dm { user_id: Uuid, name: String },
    #[serde(rename_all = "camelCase")]
    Community { community_id: Uuid, name: String },
    #[serde(rename_all = "camelCase")]
    Channel {
        community_id: Uuid,
        community_name: String,
        channel_id: Uuid,
        name: String,
        voice: bool,
    },
    #[serde(rename_all = "camelCase")]
    Message {
        message_id: Uuid,
        author_id: Uuid,
        author_name: String,
        snippet: String,
        created_at: DateTime<Utc>,
        place: SearchPlaceResponse,
    },
}

impl From<HistoryTarget> for HistoryTargetResponse {
    fn from(target: HistoryTarget) -> Self {
        match target {
            HistoryTarget::Query { text } => Self::Query { text },
            HistoryTarget::Person { user_id, name } => Self::Person { user_id, name },
            HistoryTarget::Dm { user_id, name } => Self::Dm { user_id, name },
            HistoryTarget::Community { community_id, name } => Self::Community { community_id, name },
            HistoryTarget::Channel { community_id, community_name, channel_id, name, voice } => {
                Self::Channel { community_id, community_name, channel_id, name, voice }
            }
            HistoryTarget::Message { message_id, author_id, author_name, snippet, created_at, place } => Self::Message {
                message_id,
                author_id,
                author_name,
                snippet,
                created_at,
                place: place.into(),
            },
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryItemResponse {
    id: Uuid,
    used_at: DateTime<Utc>,
    #[serde(flatten)]
    target: HistoryTargetResponse,
}

impl From<HistoryItem> for HistoryItemResponse {
    fn from(item: HistoryItem) -> Self {
        Self { id: item.id, used_at: item.used_at, target: item.target.into() }
    }
}

#[derive(Debug, serde::Serialize)]
struct HistoryResponse {
    items: Vec<HistoryItemResponse>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordHistoryBody {
    kind: String,
    query: Option<String>,
    target_id: Option<Uuid>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordedHistoryResponse {
    id: Uuid,
    used_at: DateTime<Utc>,
}

#[get("/search/history")]
async fn list_search_history(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    let items = app_state.search_history_usecase.list(UserId::new(auth_token.id)).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(HistoryResponse {
        items: items.into_iter().map(Into::into).collect(),
    })))
}

#[post("/search/history")]
async fn record_search_history(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<RecordHistoryBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    let recorded = app_state
        .search_history_usecase
        .record(UserId::new(auth_token.id), &body.kind, body.query.as_deref(), body.target_id)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(RecordedHistoryResponse {
        id: recorded.id,
        used_at: recorded.used_at,
    })))
}

#[delete("/search/history/{id}")]
async fn remove_search_history_item(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    app_state
        .search_history_usecase
        .remove(UserId::new(auth_token.id), path.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/search/history")]
async fn clear_search_history(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    app_state.search_history_usecase.clear(UserId::new(auth_token.id)).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn search_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(search_people)
        .service(list_search_history)
        .service(record_search_history)
        .service(remove_search_history_item)
        .service(clear_search_history)
        .service(global_search);
}
