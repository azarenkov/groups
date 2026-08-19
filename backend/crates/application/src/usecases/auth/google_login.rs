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
        name::{ReservedNames, UserName, UserNameError},
        repository::UserRepository,
    },
};

use crate::{
    dto::requests::{
        auth::{google_complete::GoogleCompleteRequest, google_login::GoogleLoginRequest},
        client_context::ClientContext,
    },
    ports::{
        hasher::{HashError, Hasher},
        oauth_verifier::{OAuthIdentity, OAuthIdentityVerifier, OAuthProvider, OAuthVerifyError},
        text_moderation::{TextModeration, TextModerationError},
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
    #[error("This username is already taken")]
    UsernameTaken,
    #[error("Invalid username: {0}")]
    InvalidName(#[from] UserNameError),
    #[error("Username is not allowed")]
    NameNotAllowed,
    #[error("Username moderation unavailable: {0}")]
    ModerationUnavailable(String),
}

pub enum GoogleLoginOutcome {
    Authenticated(IssuedTokens),
    NeedsUsername,
}

pub struct GoogleLogin<U, R, I, V, H, TM>
where
    U: UserRepository,
    R: RefreshTokenRepository,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
    H: Hasher,
    TM: TextModeration,
{
    user_repository: Arc<U>,
    refresh_token_repository: Arc<R>,
    token_issuer: Arc<I>,
    verifier: Arc<V>,
    hasher: Arc<H>,
    text_moderation: Arc<TM>,
    reserved_names: Arc<ReservedNames>,
}

impl<U, R, I, V, H, TM> GoogleLogin<U, R, I, V, H, TM>
where
    U: UserRepository,
    R: RefreshTokenRepository,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
    H: Hasher,
    TM: TextModeration,
{
    pub fn new(
        user_repository: Arc<U>,
        refresh_token_repository: Arc<R>,
        token_issuer: Arc<I>,
        verifier: Arc<V>,
        hasher: Arc<H>,
        text_moderation: Arc<TM>,
        reserved_names: Arc<ReservedNames>,
    ) -> Self {
        Self {
            user_repository,
            refresh_token_repository,
            token_issuer,
            verifier,
            hasher,
            text_moderation,
            reserved_names,
        }
    }

    pub async fn execute(
        &self,
        request: GoogleLoginRequest,
    ) -> Result<GoogleLoginOutcome, GoogleLoginError> {
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
            let tokens = self
                .issue_session(user_id, request.client, request.ctx.request_id)
                .await?;
            return Ok(GoogleLoginOutcome::Authenticated(tokens));
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
                    if self.user_repository.has_inheritable_state(&user_id).await? {
                        return Err(GoogleLoginError::AccountConflict);
                    }
                    let random_secret = uuid::Uuid::new_v4().to_string();
                    let password_hash = PasswordHash::new(self.hasher.hash(&random_secret)?);
                    self.user_repository
                        .claim_with_oauth(
                            &user_id,
                            &password_hash,
                            Utc::now(),
                            provider_str,
                            &identity.subject,
                            &email,
                        )
                        .await?;
                }

                let tokens = self
                    .issue_session(user_id, request.client, request.ctx.request_id)
                    .await?;
                Ok(GoogleLoginOutcome::Authenticated(tokens))
            }
            None => Ok(GoogleLoginOutcome::NeedsUsername),
        }
    }

    pub async fn complete(
        &self,
        request: GoogleCompleteRequest,
    ) -> Result<IssuedTokens, GoogleLoginError> {
        let identity = self
            .verifier
            .verify(OAuthProvider::Google, &request.id_token)
            .await?;
        if !identity.email_verified {
            return Err(GoogleLoginError::EmailNotVerified);
        }

        let provider_str = identity.provider.as_str();
        let email = Email::new(identity.email.clone());

        if let Some(user_id) = self
            .user_repository
            .find_user_id_by_oauth(provider_str, &identity.subject)
            .await?
        {
            return self
                .issue_session(user_id, request.client, request.ctx.request_id)
                .await;
        }

        if self.user_repository.find_by_email(&email).await?.is_some() {
            return Err(GoogleLoginError::AccountConflict);
        }

        let username = UserName::parse(&request.username, &self.reserved_names)?;
        if self
            .user_repository
            .find_by_name(&username)
            .await?
            .is_some()
        {
            return Err(GoogleLoginError::UsernameTaken);
        }

        match self.text_moderation.ensure_allowed(username.as_str()).await {
            Ok(()) => {}
            Err(TextModerationError::Rejected) => {
                return Err(GoogleLoginError::NameNotAllowed);
            }
            Err(TextModerationError::Unavailable(e)) => {
                return Err(GoogleLoginError::ModerationUnavailable(e));
            }
        }

        self.register_with_oauth(
            username,
            email,
            &identity,
            request.locale,
            request.client,
            request.ctx.request_id,
        )
        .await
    }

    async fn register_with_oauth(
        &self,
        username: UserName,
        email: Email,
        identity: &OAuthIdentity,
        locale: domain::user::locale::Locale,
        client: ClientContext,
        request_id: Option<String>,
    ) -> Result<IssuedTokens, GoogleLoginError> {
        let random_secret = uuid::Uuid::new_v4().to_string();
        let password_hash = PasswordHash::new(self.hasher.hash(&random_secret)?);

        let name_check = username.clone();
        let mut user = User::create(
            username,
            email.clone(),
            password_hash,
            client.ip.clone(),
            client.user_agent.clone(),
            locale,
        );
        user.verify_email(Utc::now());

        let user_id = user.id_ownned();
        let tokens = self.token_issuer.issue(&user_id, true).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user_id,
            tokens.refresh_token_expires_at,
            client.ip,
            client.user_agent,
        );

        let records: Vec<OutboxRecord> = user
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        match self
            .user_repository
            .register_user(&user, &refresh_token, &records)
            .await
        {
            Ok(()) => {}
            Err(RepositoryError::AlreadyExists) => {
                if self.user_repository.find_by_name(&name_check).await?.is_some() {
                    return Err(GoogleLoginError::UsernameTaken);
                }
                return Err(GoogleLoginError::Repository(RepositoryError::AlreadyExists));
            }
            Err(e) => return Err(e.into()),
        }

        self.user_repository
            .link_oauth_identity(
                identity.provider.as_str(),
                &identity.subject,
                &user_id,
                &email,
            )
            .await?;

        Ok(tokens)
    }

    async fn issue_session(
        &self,
        user_id: UserId,
        client: ClientContext,
        request_id: Option<String>,
    ) -> Result<IssuedTokens, GoogleLoginError> {
        let tokens = self.token_issuer.issue(&user_id, true).await?;

        let refresh_token = RefreshToken::issue(
            tokens.refresh_token_id,
            user_id,
            tokens.refresh_token_expires_at,
            client.ip.clone(),
            client.user_agent.clone(),
        );

        let event = UserEvent::LoggedIn(UserLoggedIn::record(
            user_id,
            tokens.refresh_token_id,
            client.ip,
            client.user_agent,
            Utc::now(),
        ));
        let records: Vec<OutboxRecord> = vec![event.to_outbox_record_with_request_id(request_id)?];

        self.refresh_token_repository
            .save_refresh_token_with_events(&refresh_token, &records)
            .await?;

        Ok(tokens)
    }
}
