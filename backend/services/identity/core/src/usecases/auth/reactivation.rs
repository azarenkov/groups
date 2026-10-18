use chrono::{DateTime, Duration, Utc};
use domain::user::id::UserId;
use identity_domain::{refresh_token::RefreshToken, unit_of_work::IdentityUnitOfWork};
use ports::repository::RepositoryError;

use crate::ports::{access_revoker::AccessRevoker, token_issuer::{IssuedTokens, TokenIssuer, TokenIssuerError}};

pub async fn wait_past_cutoff(cutoff: DateTime<Utc>) {
    let next_second = cutoff + Duration::seconds(1);
    let now = Utc::now();
    if next_second > now
        && let Ok(wait) = (next_second - now).to_std()
    {
        tokio::time::sleep(wait).await;
    }
}

pub struct SessionClient {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

pub async fn issue_session_after_cutoff<W, I, E>(
    unit_of_work: &W,
    token_issuer: &I,
    user_id: UserId,
    verified: bool,
    cutoff: DateTime<Utc>,
    client: SessionClient,
) -> Result<IssuedTokens, E>
where
    W: IdentityUnitOfWork,
    I: TokenIssuer,
    E: From<RepositoryError> + From<TokenIssuerError>,
{
    wait_past_cutoff(cutoff).await;
    let tokens = token_issuer.issue(&user_id, verified).await?;
    let refresh_token = RefreshToken::issue(
        tokens.refresh_token_id,
        user_id,
        tokens.refresh_token_expires_at,
        client.ip,
        client.user_agent,
    );
    let mut tx = unit_of_work.begin().await?;
    tx.insert_refresh_token(&refresh_token).await?;
    tx.commit().await?;
    Ok(tokens)
}

pub async fn revoke_access_after_commit<R>(revoker: &R, user_id: &UserId, cutoff: DateTime<Utc>)
where
    R: AccessRevoker + ?Sized,
{
    if let Err(e) = revoker.revoke_access_issued_before(user_id, cutoff).await {
        tracing::error!(error = %e, user_id = %user_id.uuid(), "Failed to revoke access tokens after the credential change was committed; they expire with their own TTL");
    }
}
