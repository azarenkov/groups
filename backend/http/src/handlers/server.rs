use actix_web::{HttpResponse, Responder, delete, get, post, web};
use domain::{
    server::id::ServerId, shared::pagination::Pagination,
    user::id::UserId,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    middleware::auth::AuthToken,
    shared::{
        dto::{
            pagination::{PaginationMeta, PaginationQuery},
            requests::server::create_server::CreateServerRequest,
            responses::{
                ApiResponse,
                server::{
                    create_server::CreateServerResponse,
                    list_server_members::ListServerMembersResponse,
                    list_server_channels::ListServerChannelsResponse,
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
    query: web::Query<PaginationQuery>,
) -> Result<impl Responder, ApiError> {
    query.validate()?;

    let user_id = UserId::new(auth_token.id);
    let pagination = Pagination::new(query.page, query.per_page);

    let result = app_state
        .list_servers_by_user_usecase()
        .execute(user_id, pagination)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let response = ListServersByUserResponse::new(result.items, pagination_meta);

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/{server_id}/members")]
async fn list_server_members(
    _auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<PaginationQuery>,
) -> Result<impl Responder, ApiError> {
    query.validate()?;

    let server_id = ServerId::new(path.into_inner());
    let pagination = Pagination::new(query.page, query.per_page);

    let result = app_state
        .list_server_members_usecase()
        .execute(server_id, pagination)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let response = ListServerMembersResponse::new(result.items, pagination_meta);

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

#[get("/{server_id}/channels")]
async fn list_server_channels(
    _auth_token: AuthToken,
    app_state: web::Data<AppState>,
    path: web::Path<Uuid>,
    query: web::Query<PaginationQuery>,
) -> Result<impl Responder, ApiError> {
    query.validate()?;

    let server_id = ServerId::new(path.into_inner());
    let pagination = Pagination::new(query.page, query.per_page);

    let result = app_state
        .list_server_rooms_usecase()
        .execute(server_id, pagination)
        .await?;

    let pagination_meta = PaginationMeta::new(result.total, query.page, query.per_page);
    let response = ListServerChannelsResponse::new(result.items, pagination_meta);

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
