use std::sync::Arc;

use identity_domain::user::User;

use crate::ports::hasher::{HashError, Hasher};

#[derive(Debug, Clone)]
pub struct Reauth {
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ReauthError {
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Password is incorrect")]
    InvalidPassword,
}

pub struct Reauthenticator<H>
where
    H: Hasher,
{
    hasher: Arc<H>,
}

impl<H> Reauthenticator<H>
where
    H: Hasher,
{
    pub fn new(hasher: Arc<H>) -> Self {
        Self { hasher }
    }

    pub async fn verify(&self, user: &User, reauth: &Reauth) -> Result<(), ReauthError> {
        if self.hasher.verify(&reauth.password, user.password_hash().as_str())? {
            Ok(())
        } else {
            Err(ReauthError::InvalidPassword)
        }
    }
}
