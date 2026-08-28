use std::sync::Arc;

use identity_domain::{user::User, user::repository::UserRepository};
use ports::repository::RepositoryError;

use crate::ports::{
    hasher::{HashError, Hasher},
    oauth_verifier::{OAuthIdentityVerifier, OAuthProvider, OAuthVerifyError},
};

#[derive(Debug, Clone)]
pub enum Reauth {
    Password(String),
    GoogleIdToken(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ReauthError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("OAuth verification failed: {0}")]
    Verify(#[from] OAuthVerifyError),
    #[error("Password is incorrect")]
    InvalidPassword,
    #[error("This Google account is not linked to the user")]
    OAuthNotLinked,
}

pub struct Reauthenticator<U, H, V>
where
    U: UserRepository,
    H: Hasher,
    V: OAuthIdentityVerifier,
{
    user_repository: Arc<U>,
    hasher: Arc<H>,
    verifier: Arc<V>,
}

impl<U, H, V> Reauthenticator<U, H, V>
where
    U: UserRepository,
    H: Hasher,
    V: OAuthIdentityVerifier,
{
    pub fn new(user_repository: Arc<U>, hasher: Arc<H>, verifier: Arc<V>) -> Self {
        Self {
            user_repository,
            hasher,
            verifier,
        }
    }

    pub async fn verify(&self, user: &User, reauth: &Reauth) -> Result<(), ReauthError> {
        match reauth {
            Reauth::Password(password) => {
                if self.hasher.verify(password, user.password_hash().as_str())? {
                    Ok(())
                } else {
                    Err(ReauthError::InvalidPassword)
                }
            }
            Reauth::GoogleIdToken(token) => {
                let identity = self.verifier.verify(OAuthProvider::Google, token).await?;
                let linked = self
                    .user_repository
                    .find_oauth_subject_for_user(OAuthProvider::Google.as_str(), user.id())
                    .await?;
                match linked {
                    Some(subject) if subject == identity.subject => Ok(()),
                    _ => Err(ReauthError::OAuthNotLinked),
                }
            }
        }
    }
}
