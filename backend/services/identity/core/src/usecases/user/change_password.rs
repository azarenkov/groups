use std::sync::Arc;

use chrono::Utc;
use domain::{password_hash::PasswordHash, user::id::UserId};
use identity_domain::{events::IdentityEvent, unit_of_work::IdentityUnitOfWork, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::{
    ports::{
        hasher::{HashError, Hasher},
        oauth_verifier::OAuthIdentityVerifier,
        token_issuer::{TokenIssuer, TokenIssuerError},
    },
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
    pub refresh_token: Option<String>,
    pub request_id: Option<String>,
}

pub struct ChangePassword<U, W, H, I, V>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    hasher: Arc<H>,
    token_issuer: Arc<I>,
    reauthenticator: Arc<Reauthenticator<U, H, V>>,
}

impl<U, W, H, I, V> ChangePassword<U, W, H, I, V>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    H: Hasher,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
{
    pub fn new(
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        hasher: Arc<H>,
        token_issuer: Arc<I>,
        reauthenticator: Arc<Reauthenticator<U, H, V>>,
    ) -> Self {
        Self {
            user_repository,
            unit_of_work,
            hasher,
            token_issuer,
            reauthenticator,
        }
    }

    pub async fn execute(&self, user_id: UserId, request: ChangePasswordRequest) -> Result<(), ChangePasswordError> {
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
        let keep = match request.refresh_token.as_deref() {
            Some(token) => {
                let decoded = self.token_issuer.decode_refresh(token)?;
                (decoded.user_id.uuid() == user_id.uuid()).then_some(decoded.refresh_token_id)
            }
            None => None,
        };
        let new_hash = PasswordHash::new(self.hasher.hash(&request.new_password)?);

        let now = Utc::now();
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(ChangePasswordError::UserNotFound)?;
        user.change_password(new_hash, now);
        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.save_user(&user).await?;
        tx.revoke_refresh_tokens_for_user(&user_id, keep.as_ref(), now).await?;
        tx.save_events(&events, request.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
