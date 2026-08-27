use std::sync::Arc;

use chrono::Utc;
use identity_domain::{refresh_token::RefreshToken, unit_of_work::IdentityUnitOfWork, user::User, user::email::Email, user::events::UserEvent, user::events::lifecycle::UserLoggedIn, user::repository::UserRepository};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, shared::repository::error::RepositoryError, shared::value_objects::password_hash::PasswordHash, user::id::UserId, user::name::ReservedNames, user::name::UserName, user::name::UserNameError};

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
        user_state_lookup::{UserStateLookup, UserStateLookupError},
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
    #[error("User state lookup unavailable: {0}")]
    StateLookupUnavailable(#[from] UserStateLookupError),
}

pub enum GoogleLoginOutcome {
    Authenticated(IssuedTokens),
    NeedsUsername,
}

pub struct GoogleLogin<U, W, I, V, H, TM, SL>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
    H: Hasher,
    TM: TextModeration,
    SL: UserStateLookup,
{
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
    token_issuer: Arc<I>,
    verifier: Arc<V>,
    hasher: Arc<H>,
    text_moderation: Arc<TM>,
    reserved_names: Arc<ReservedNames>,
    user_state_lookup: Arc<SL>,
}

impl<U, W, I, V, H, TM, SL> GoogleLogin<U, W, I, V, H, TM, SL>
where
    U: UserRepository,
    W: IdentityUnitOfWork,
    I: TokenIssuer,
    V: OAuthIdentityVerifier,
    H: Hasher,
    TM: TextModeration,
    SL: UserStateLookup,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_repository: Arc<U>,
        unit_of_work: Arc<W>,
        token_issuer: Arc<I>,
        verifier: Arc<V>,
        hasher: Arc<H>,
        text_moderation: Arc<TM>,
        reserved_names: Arc<ReservedNames>,
        user_state_lookup: Arc<SL>,
    ) -> Self {
        Self {
            user_repository,
            unit_of_work,
            token_issuer,
            verifier,
            hasher,
            text_moderation,
            reserved_names,
            user_state_lookup,
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
                    if self
                        .user_state_lookup
                        .has_messaging_state(&user_id)
                        .await?
                    {
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
        locale: identity_domain::user::locale::Locale,
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

        let persisted = async {
            let mut tx = self.unit_of_work.begin().await?;
            tx.insert_user(&user).await?;
            tx.insert_refresh_token(&refresh_token).await?;
            tx.save_outbox(&records).await?;
            tx.commit().await
        }
        .await;
        match persisted {
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

        let mut tx = self.unit_of_work.begin().await?;
        tx.insert_refresh_token(&refresh_token).await?;
        tx.save_outbox(&records).await?;
        tx.commit().await?;

        Ok(tokens)
    }
}
