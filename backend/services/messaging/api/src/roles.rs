use actix_web::{HttpResponse, Responder, delete, get, post, put, web};
use domain::{server::id::ServerId, user::id::UserId};
use serde::{Deserialize, Serialize};
use shared::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::state::AppState;
use api_common::dto::responses::ApiResponse;
use shared::api_error::ApiError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoleItem {
    id: Uuid,
    name: String,
    color: String,
    permissions: i64,
    position: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AssignmentItem {
    user_id: Uuid,
    role_id: Uuid,
}

#[derive(Debug, Serialize)]
struct RolesResponse {
    roles: Vec<RoleItem>,
    assignments: Vec<AssignmentItem>,
}

#[get("/{server_id}/roles")]
async fn list_roles(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let server_id = ServerId::new(path.into_inner());
    let overview = app_state
        .list_roles_usecase
        .execute(server_id, user_id)
        .await?;
    let response = RolesResponse {
        roles: overview
            .roles
            .into_iter()
            .map(|r| RoleItem {
                id: r.id,
                name: r.name,
                color: r.color,
                permissions: r.permissions,
                position: r.position,
            })
            .collect(),
        assignments: overview
            .assignments
            .into_iter()
            .map(|(user_id, role_id)| AssignmentItem { user_id, role_id })
            .collect(),
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoleBody {
    name: String,
    color: String,
    permissions: i64,
}

#[post("/{server_id}/roles")]
async fn create_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<RoleBody>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let server_id = ServerId::new(path.into_inner());
    let body = body.into_inner();
    let role = app_state
        .create_role_usecase
        .execute(server_id, user_id, body.name, body.color, body.permissions)
        .await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(RoleItem {
        id: role.id,
        name: role.name,
        color: role.color,
        permissions: role.permissions,
        position: role.position,
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRoleBody {
    name: Option<String>,
    color: Option<String>,
    #[serde(default)]
    permissions_add: i64,
    #[serde(default)]
    permissions_remove: i64,
}

#[put("/{server_id}/roles/{role_id}")]
async fn update_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<UpdateRoleBody>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let (server_id, role_id) = path.into_inner();
    let body = body.into_inner();
    app_state
        .update_role_usecase
        .execute(
            ServerId::new(server_id),
            user_id,
            role_id,
            domain::server::role::RolePatch {
                name: body.name,
                color: body.color,
                add_permissions: body.permissions_add,
                remove_permissions: body.permissions_remove,
            },
        )
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{server_id}/roles/{role_id}")]
async fn delete_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let (server_id, role_id) = path.into_inner();
    app_state
        .delete_role_usecase
        .execute(ServerId::new(server_id), user_id, role_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[put("/{server_id}/members/{member_id}/roles/{role_id}")]
async fn assign_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let (server_id, member_id, role_id) = path.into_inner();
    app_state
        .assign_role_usecase
        .execute(
            ServerId::new(server_id),
            user_id,
            UserId::new(member_id),
            role_id,
        )
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{server_id}/members/{member_id}/roles/{role_id}")]
async fn revoke_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let (server_id, member_id, role_id) = path.into_inner();
    app_state
        .revoke_role_usecase
        .execute(
            ServerId::new(server_id),
            user_id,
            UserId::new(member_id),
            role_id,
        )
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{server_id}/roles/{role_a}/swap/{role_b}")]
async fn swap_roles(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);
    let (server_id, role_a, role_b) = path.into_inner();
    app_state
        .swap_roles_usecase
        .execute(ServerId::new(server_id), user_id, role_a, role_b)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

pub fn role_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_roles)
        .service(create_role)
        .service(update_role)
        .service(delete_role)
        .service(assign_role)
        .service(revoke_role)
        .service(swap_roles);
}
