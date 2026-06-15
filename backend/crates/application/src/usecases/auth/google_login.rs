use std::sync::Arc;

use chrono::Utc;
use domain::{
    refresh_token::{RefreshToken, repository::RefreshTokenRepository},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
        value_objects::password_hash::PasswordHash,
    },
    user::{
        User,
        email::Email,
        events::{UserEvent, lifecycle::UserLoggedIn},
        id::UserId,
        name::UserName,
        repository::UserRepository,
    },
};

use crate::{
    dto::requests::auth::google_login::GoogleLoginRequest,
    ports::{
        hasher::{HashError, Hasher},
        oauth_verifier::{OAuthIdentityVerifier, OAuthProvider, OAuthVerifyError},
        token_issuer::{IssuedTokens, TokenIssuer, TokenIssuerError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum GoogleLoginError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Hash error: {0}")]
    Hash(#[from] HashError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Token issuer error: {0}")]
    TokenIssuer(#[from] TokenIssuerError),
    #[error("Google token verification failed: {0}")]
    Verify(#[from] OAuthVerifyError),
    #[error("Google account email is not verified")]
    EmailNotVerified,
    #[error("This email is already linked to a different Google account")]
    AccountConflict,
}

pub struct GoogleLogin<U, R, I, V, H>
where
    U: UserRepository,
    R: RefreshTokenRepository,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
    H: Hasher,
{
    user_repository: Arc<U>,
    refresh_token_repository: Arc<R>,
    token_issuer: Arc<I>,
    verifier: Arc<V>,
    hasher: Arc<H>,
}

impl<U, R, I, V, H> GoogleLogin<U, R, I, V, H>
where
    U: UserRepository,
    R: RefreshTokenRepository,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
    H: Hasher,
{
    pub fn new(
        user_repository: Arc<U>,
        refresh_token_repository: Arc<R>,
        token_issuer: Arc<I>,
        verifier: Arc<V>,
        hasher: Arc<H>,
    ) -> Self {
        Self {
            user_repository,
            refresh_token_repository,
            token_issuer,
            verifier,
            hasher,
        }
    }

    pub async fn execute(
        &self,
        request: GoogleLoginRequest,
    ) -> Result<IssuedTokens, GoogleLoginError> {
        let identity = self
            .verifier
            .verify(OAuthProvider::Google, &request.id_token)
            .await?;
        if !identity.email_verified {
            return Err(GoogleLoginError::EmailNotVerified);
        }

        let provider = identity.provider;
        let provider_str = provider.as_str();
        let email = Email::new(identity.email.clone());

        if let Some(user_id) = self
            .user_repository
            .find_user_id_by_oauth(provider_str, &identity.subject)
            .await?
        {
            return self.issue_session(user_id, request).await;
        }

        match self.user_repository.find_by_email(&email).await? {
            Some(user) => {
                let user_id = user.id_ownned();

                if self
                    .user_repository
                    .find_oauth_subject_for_user(provider_str, &user_id)
                    .await?
                    .is_some()
                {
                    return Err(GoogleLoginError::AccountConflict);
                }

                if user.is_email_verified() {
                    self.user_repository
                        .link_oauth_identity(provider_str, &identity.subject, &user_id, &email)
                        .await?;
                } else {
                    let random_secret = uuid::Uuid::new_v4().to_string();
                    let password_hash = PasswordHash::new(self.hasher.hash(&random_secret)?);
                    self.refresh_token_repository
                        .revoke_all_for_user(&user_id)
                        .await?;
                    self.user_repository
                        .reset_password_and_verify(&user_id, &password_hash, Utc::now())
                        .await?;
                    self.user_repository
                        .link_oauth_identity(provider_str, &identity.subject, &user_id, &email)
                        .await?;
                }

                self.issue_session(user_id, request).await
            }
            None => {
                let request_id = request.ctx.request_id.clone();
                let username = self
                    .allocate_username(&identity.name, &identity.email)
                    .await?;
                let random_secret = uuid::Uuid::new_v4().to_string();
                let password_hash = PasswordHash::new(self.hasher.hash(&random_secret)?);

                let mut user = User::create(
                    UserName::new(username),
                    email.clone(),
                    password_hash,
                    request.client.ip.clone(),
                    request.client.user_agent.clone(),
                    request.locale,
                );
                user.verify_email(Utc::now());

                let user_id = user.id_ownned();
                let tokens = self.token_issuer.issue(&user_id, true).await?;

                let refresh_token = RefreshToken::issue(
                    tokens.refresh_token_id,
                    user_id,
                    tokens.refresh_token_expires_at,
                    request.client.ip,
                    request.client.user_agent,
                );

                let records: Vec<OutboxRecord> = user
                    .pull_events()
                    .iter()
                    .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
                    .collect::<Result<Vec<_>, _>>()?;

                self.user_repository
                    .register_user(&user, &refresh_token, &records)
                    .await?;

                self.user_repository
                    .link_oauth_identity(provider_str, &identity.subject, &user_id, &email)
                    .await?;

                Ok(tokens)
            }
        }
    }

    async fn issue_session(
        &self,
        user_id: UserId,
        request: GoogleLoginRequest,
    ) -> Result<IssuedTokens, GoogleLoginError> {
        let tokens = self.token_issuer.issue(&user_id, true).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user_id,
            tokens.refresh_token_expires_at,
            request.client.ip.clone(),
            request.client.user_agent.clone(),
        );

        let event = UserEvent::LoggedIn(UserLoggedIn::record(
            user_id,
            tokens.refresh_token_id,
            request.client.ip,
            request.client.user_agent,
            Utc::now(),
        ));
        let records: Vec<OutboxRecord> =
            vec![event.to_outbox_record_with_request_id(request.ctx.request_id)?];

        self.refresh_token_repository
            .save_refresh_token_with_events(&refresh_token, &records)
            .await?;

        Ok(tokens)
    }

    async fn allocate_username(
        &self,
        name: &Option<String>,
        email: &str,
    ) -> Result<String, GoogleLoginError> {
        let base = sanitize_username(name.as_deref().unwrap_or(""))
            .or_else(|| sanitize_username(email.split('@').next().unwrap_or("")))
            .unwrap_or_else(|| "user".to_string());

        if self
            .user_repository
            .find_by_name(&UserName::new(base.clone()))
            .await?
            .is_none()
        {
            return Ok(base);
        }

        for _ in 0..6 {
            let suffix = &uuid::Uuid::new_v4().simple().to_string()[..4];
            let candidate = format!("{}{}", truncate(&base, 10), suffix);
            if self
                .user_repository
                .find_by_name(&UserName::new(candidate.clone()))
                .await?
                .is_none()
            {
                return Ok(candidate);
            }
        }

        Ok(format!("user{}", &uuid::Uuid::new_v4().simple().to_string()[..8]))
    }
}

fn sanitize_username(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                Some(c)
            } else if c == ' ' || c == '.' || c == '-' {
                Some('_')
            } else {
                None
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.len() < 3 {
        return None;
    }
    Some(truncate(trimmed, 15))
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
