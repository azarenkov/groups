use std::sync::Arc;

use chrono::Utc;
use identity_domain::{refresh_token::repository::RefreshTokenRepository, user::events::UserEvent, user::events::lifecycle::UserLoggedOut};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, shared::repository::error::RepositoryError, user::id::UserId};

use crate::{
    dto::requests::auth::logout::LogoutRequest,
    ports::token_issuer::{TokenIssuer, TokenIssuerError},
};

#[derive(Debug, thiserror::Error)]
pub enum LogoutError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
    #[error("Refresh token not found")]
    RefreshTokenNotFound,
    #[error("Refresh token does not belong to user")]
    UserMismatch,
}

pub struct Logout<R, I>
where
    R: RefreshTokenRepository,
    I: TokenIssuer,
{
    refresh_token_repository: Arc<R>,
    token_issuer: Arc<I>,
}

impl<R, I> Logout<R, I>
where
    R: RefreshTokenRepository,
    I: TokenIssuer,
{
    pub fn new(refresh_token_repository: Arc<R>, token_issuer: Arc<I>) -> Self {
        Self {
            refresh_token_repository,
            token_issuer,
        }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        request: LogoutRequest,
    ) -> Result<(), LogoutError> {
        let decoded = self.token_issuer.decode_refresh(&request.refresh_token)?;

        if decoded.user_id.uuid() != user_id.uuid() {
            return Err(LogoutError::UserMismatch);
        }

        let mut refresh_token = self
            .refresh_token_repository
            .find_by_id(&decoded.refresh_token_id)
            .await?
            .ok_or(LogoutError::RefreshTokenNotFound)?;

        if refresh_token.user_id().uuid() != user_id.uuid() {
            return Err(LogoutError::UserMismatch);
        }

        if refresh_token.is_revoked() {
            return Ok(());
        }

        let now = Utc::now();
        refresh_token.revoke(now);

        let event = UserEvent::LoggedOut(UserLoggedOut::record(
            user_id,
            decoded.refresh_token_id,
            request.client.ip,
            request.client.user_agent,
            now,
        ));
        let records: Vec<OutboxRecord> =
            vec![event.to_outbox_record_with_request_id(request.ctx.request_id)?];

        self.refresh_token_repository
            .revoke_with_events(&refresh_token, &records)
            .await?;

        Ok(())
    }
}
