use actix_web::{HttpResponse, Responder, delete, get, post, web};
use domain::{server::value_objects::id::ServerId, user::value_objects::id::UserId};
use uuid::Uuid;
use validator::Validate;

use crate::{
    middleware::auth::AuthToken,
    shared::{
        dto::{
            requests::server::create_server::CreateServerRequest,
            responses::{
                ApiResponse,
                server::{
                    create_server::CreateServerResponse,
                    list_server_members::ListServerMembersResponse,
                    list_server_rooms::ListServerRoomsResponse,
                    list_servers_by_user::ListServersByUserResponse,
                },
            },
        },
        error::ApiError,
        state::AppState,
    },
};

#[post("/create")]
async fn create_server(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    request: web::Json<CreateServerRequest>,
) -> Result<impl Responder, ApiError> {
    let request: CreateServerRequest = request.into_inner();
    request.validate()?;

    let server = app_state
        .create_server_usecase()
        .execute(request.into(), UserId::new(auth_token.id))
        .await?;

    let response = CreateServerResponse::from(server);

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

#[get("/list")]
async fn list_servers_by_user(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    let user_id = UserId::new(auth_token.id);

    let servers = app_state
        .list_servers_by_user_usecase()
        .execute(user_id)
        .await?;

    let response = ListServersByUserResponse::new(servers);

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/{server_id}/members")]
async fn list_server_members(
    _auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());

    let members = app_state
        .list_server_members_usecase()
        .execute(server_id)
        .await?;

    let response = ListServerMembersResponse::new(members);

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/{server_id}/rooms")]
async fn list_server_rooms(
    _auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());

    let rooms = app_state
        .list_server_rooms_usecase()
        .execute(server_id)
        .await?;

    let response = ListServerRoomsResponse::new(rooms);

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[delete("/{server_id}")]
async fn delete_server(
    auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<impl Responder, ApiError> {
    let server_id = ServerId::new(path.into_inner());
    let owner_id = UserId::new(auth_token.id);

    let deleted = app_state
        .delete_server_usecase()
        .execute(server_id, owner_id)
        .await?;

    if deleted {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}
