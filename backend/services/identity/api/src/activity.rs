use actix_web::{HttpResponse, Responder, delete, get, put, web};
use api_common::dto::responses::ApiResponse;
use api_common::middleware::auth::RateLimitedAuthToken;
use chrono::{DateTime, Utc};
use identity::usecases::user::activity::ActivityInput;
use identity_domain::user::id::UserId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::http_errors::HttpError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityBody {
    game_id: String,
    name: String,
    started_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityItem {
    user_id: Uuid,
    game_id: String,
    name: String,
    started_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivitiesResponse {
    activities: Vec<ActivityItem>,
}

#[derive(Debug, Deserialize)]
struct ActivityQuery {
    #[serde(default)]
    ids: String,
}

#[put("/me/activity")]
pub async fn set_me_activity(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    body: web::Json<ActivityBody>,
) -> Result<impl Responder, HttpError> {
    let body = body.into_inner();
    let activity = app_state
        .set_activity_usecase
        .execute(
            UserId::new(auth_token.id),
            ActivityInput {
                game_id: body.game_id,
                name: body.name,
                started_at: body.started_at,
            },
        )
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ActivityItem {
        user_id: auth_token.id,
        game_id: activity.game_id,
        name: activity.name,
        started_at: activity.started_at,
    })))
}

#[delete("/me/activity")]
pub async fn clear_me_activity(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, HttpError> {
    app_state
        .clear_activity_usecase
        .execute(UserId::new(auth_token.id))
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("/activity")]
pub async fn list_activities(
    _auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    query: web::Query<ActivityQuery>,
) -> Result<impl Responder, HttpError> {
    let ids: Vec<UserId> = query
        .ids
        .split(',')
        .filter_map(|raw| Uuid::parse_str(raw.trim()).ok())
        .map(UserId::new)
        .collect();
    let activities = app_state.get_activities_usecase.execute(ids).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ActivitiesResponse {
        activities: activities
            .into_iter()
            .map(|(user_id, a)| ActivityItem {
                user_id: user_id.uuid(),
                game_id: a.game_id,
                name: a.name,
                started_at: a.started_at,
            })
            .collect(),
    })))
}
