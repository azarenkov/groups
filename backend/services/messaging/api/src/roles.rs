use actix_web::{HttpResponse, Responder, delete, get, post, put, web};
use messaging_domain::community::id::CommunityId;
use domain::user::id::UserId;
use serde::{Deserialize, Serialize};
use api_common::middleware::auth::RateLimitedAuthToken;
use uuid::Uuid;

use crate::state::AppState;
use api_common::dto::responses::ApiResponse;
use crate::http_errors::HttpError;
use messaging_domain::community::role::{Permission, Permissions};

fn parse_permission_list(names: &[String]) -> Result<Permissions, HttpError> {
    names
        .iter()
        .map(|name| {
            serde_json::from_value::<Permission>(serde_json::Value::String(name.clone())).map_err(|_| {
                HttpError::new(
                    format!("Unknown permission: {name}"),
                    actix_web::http::StatusCode::BAD_REQUEST,
                    "VALIDATION_FAILED",
                )
            })
        })
        .collect()
}

fn requested(list: Option<&[String]>, mask: i64) -> Result<Permissions, HttpError> {
    match list {
        Some(names) => parse_permission_list(names),
        None => Ok(Permissions::from_bits_retain(mask).known()),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RoleItem {
    id: Uuid,
    name: String,
    color: String,
    permissions: i64,
    permission_list: Vec<Permission>,
    position: i32,
}

impl RoleItem {
    fn from_role(role: messaging_domain::community::role::CommunityRole) -> Self {
        Self {
            id: role.id,
            name: role.name,
            color: role.color,
            permissions: role.permissions.bits(),
            permission_list: role.permissions.iter().collect(),
            position: role.position,
        }
    }
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

#[get("/{community_id}/roles")]
async fn list_roles(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let community_id = CommunityId::new(path.into_inner());
    let overview = app_state
        .list_roles_usecase
        .execute(community_id, user_id)
        .await?;
    let response = RolesResponse {
        roles: overview
            .roles
            .into_iter()
            .map(RoleItem::from_role)
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
    #[serde(default)]
    permissions: i64,
    permission_list: Option<Vec<String>>,
}

#[post("/{community_id}/roles")]
async fn create_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    body: web::Json<RoleBody>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let community_id = CommunityId::new(path.into_inner());
    let body = body.into_inner();
    let permissions = requested(body.permission_list.as_deref(), body.permissions)?;
    let role = app_state
        .create_role_usecase
        .execute(community_id, user_id, body.name, body.color, permissions)
        .await?;
    Ok(HttpResponse::Created().json(ApiResponse::success(RoleItem::from_role(role))))
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
    add_permission_list: Option<Vec<String>>,
    remove_permission_list: Option<Vec<String>>,
}

#[put("/{community_id}/roles/{role_id}")]
async fn update_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<UpdateRoleBody>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let (community_id, role_id) = path.into_inner();
    let body = body.into_inner();
    let add_permissions = requested(body.add_permission_list.as_deref(), body.permissions_add)?;
    let remove_permissions =
        requested(body.remove_permission_list.as_deref(), body.permissions_remove)?;
    app_state
        .update_role_usecase
        .execute(
            CommunityId::new(community_id),
            user_id,
            role_id,
            messaging_domain::community::role::RolePatch {
                name: body.name,
                color: body.color,
                add_permissions,
                remove_permissions,
            },
        )
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{community_id}/roles/{role_id}")]
async fn delete_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let (community_id, role_id) = path.into_inner();
    app_state
        .delete_role_usecase
        .execute(CommunityId::new(community_id), user_id, role_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[put("/{community_id}/members/{member_id}/roles/{role_id}")]
async fn assign_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let (community_id, member_id, role_id) = path.into_inner();
    app_state
        .assign_role_usecase
        .execute(
            CommunityId::new(community_id),
            user_id,
            UserId::new(member_id),
            role_id,
        )
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[delete("/{community_id}/members/{member_id}/roles/{role_id}")]
async fn revoke_role(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let (community_id, member_id, role_id) = path.into_inner();
    app_state
        .revoke_role_usecase
        .execute(
            CommunityId::new(community_id),
            user_id,
            UserId::new(member_id),
            role_id,
        )
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{community_id}/roles/{role_a}/swap/{role_b}")]
async fn swap_roles(
    auth_token: RateLimitedAuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl Responder, HttpError> {
    let user_id = UserId::new(auth_token.id);
    let (community_id, role_a, role_b) = path.into_inner();
    app_state
        .swap_roles_usecase
        .execute(CommunityId::new(community_id), user_id, role_a, role_b)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_list_wins_over_the_old_mask() {
        let names = vec!["muteMembers".to_string(), "kickMembers".to_string()];
        let parsed = requested(Some(&names), 1).unwrap();
        assert_eq!(
            parsed.iter().collect::<Vec<_>>(),
            vec![Permission::MuteMembers, Permission::KickMembers]
        );
    }

    #[test]
    fn old_clients_still_send_a_mask() {
        let parsed = requested(None, Permission::ManageChannels.bit() | 4096).unwrap();
        assert_eq!(parsed, Permissions::from(Permission::ManageChannels));
    }

    #[test]
    fn an_unknown_name_is_rejected() {
        let names = vec!["flyAround".to_string()];
        assert!(requested(Some(&names), 0).is_err());
    }

    #[test]
    fn a_role_is_sent_with_both_the_mask_and_the_names() {
        let role = messaging_domain::community::role::CommunityRole {
            id: Uuid::nil(),
            community_id: Uuid::nil(),
            name: "mods".into(),
            color: "#000000".into(),
            permissions: [Permission::MuteMembers, Permission::DeafenMembers].into_iter().collect(),
            position: 1,
            created_at: chrono::Utc::now(),
        };
        let json = serde_json::to_value(RoleItem::from_role(role)).unwrap();
        assert_eq!(json["permissions"], 8 | 128);
        assert_eq!(json["permissionList"], serde_json::json!(["muteMembers", "deafenMembers"]));
    }
}
