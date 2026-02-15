use actix_web::{HttpResponse, Responder, delete, post, web};
use domain::{server::value_objects::id::ServerId, user::value_objects::id::UserId};
use uuid::Uuid;
use validator::Validate;

use crate::{
    middleware::auth::AuthToken,
    shared::{
        dto::{
            requests::server::create_server::CreateServerRequest,
            responses::{ApiResponse, server::create_server::CreateServerResponse},
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
