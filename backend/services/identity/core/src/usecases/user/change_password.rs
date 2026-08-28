use std::sync::Arc;

use chrono::Utc;
use domain::{password_hash::PasswordHash, user::id::UserId};
use identity_domain::{events::IdentityEvent, refresh_token::RefreshToken, unit_of_work::IdentityUnitOfWork, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        access_revoker::{AccessRevoker, AccessRevokerError},
        hasher::{HashError, Hasher},
        oauth_verifier::OAuthIdentityVerifier,
        token_issuer::{IssuedTokens, TokenIssuer, TokenIssuerError},
    },
    usecases::auth::reactivation::wait_past_cutoff,
    usecases::user::reauth::{Reauth, ReauthError, Reauthenticator},
};

#[derive(Debug, thiserror::Error)]
pub enum ChangePasswordError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
    #[error("Access revocation error: {0}")]
    AccessRevoker(#[from] AccessRevokerError),
    #[error("Re-authentication failed: {0}")]
    Reauth(#[from] ReauthError),
    #[error("New password must differ from the current one")]
    SamePassword,
    #[error("User not found")]
    UserNotFound,
}

pub struct ChangePasswordRequest {
    pub reauth: Reauth,
    pub new_password: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

pub struct ChangePassword<U, W, H, I, V, R>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
    R: AccessRevoker,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
    token_issuer: Arc<I>,
    reauthenticator: Arc<Reauthenticator<U, H, V>>,
    access_revoker: Arc<R>,
}

impl<U, W, H, I, V, R> ChangePassword<U, W, H, I, V, R>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
    R: AccessRevoker,
{
    pub fn new(
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        hasher: Arc<H>,
        token_issuer: Arc<I>,
        reauthenticator: Arc<Reauthenticator<U, H, V>>,
        access_revoker: Arc<R>,
    ) -> Self {
        Self {
            user_repository,
            unit_of_work,
            hasher,
            token_issuer,
            reauthenticator,
            access_revoker,
        }
    }

    pub async fn execute(&self, user_id: UserId, request: ChangePasswordRequest) -> Result<IssuedTokens, ChangePasswordError> {
        let user = self
            .user_repository
            .find_by_id(&user_id)
            .await?
            .ok_or(ChangePasswordError::UserNotFound)?;
        if user.is_deleted() {
            return Err(ChangePasswordError::UserNotFound);
        }
        self.reauthenticator.verify(&user, &request.reauth).await?;
        if self
            .hasher
            .verify(&request.new_password, user.password_hash().as_str())?
        {
            return Err(ChangePasswordError::SamePassword);
        }
        let new_hash = PasswordHash::new(self.hasher.hash(&request.new_password)?);

        let snapshot = user;
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(ChangePasswordError::UserNotFound)?;
        if user.is_deleted() || user.is_disabled() {
            return Err(ChangePasswordError::UserNotFound);
        }
        if user.password_hash().as_str() != snapshot.password_hash().as_str()
            || user.updated_at() != snapshot.updated_at()
        {
            self.reauthenticator.verify(&user, &request.reauth).await?;
        }
        let now = Utc::now();
        let cutoff = now;
        user.change_password(new_hash, now);
        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.save_user(&user).await?;
        tx.revoke_refresh_tokens_for_user(&user_id, None, now).await?;
        wait_past_cutoff(cutoff).await;
        let tokens = self.token_issuer.issue(&user_id, user.is_verified()).await?;
        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user_id,
            tokens.refresh_token_expires_at,
            request.ip,
            request.user_agent,
        );
        tx.insert_refresh_token(&refresh_token).await?;
        tx.save_events(&events, request.request_id.as_deref()).await?;
        self.access_revoker.revoke_access_issued_before(&user_id, cutoff).await?;
        tx.commit().await?;
        Ok(tokens)
    }
}
