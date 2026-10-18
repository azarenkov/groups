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

#[derive(Debug, thiserror::Error)]
pub enum SessionRotationError {
    #[error("Credentials changed concurrently; sign in again")]
    Superseded,
}

fn same_stored_instant(a: DateTime<Utc>, b: DateTime<Utc>) -> bool {
    a.timestamp_micros() == b.timestamp_micros()
}

pub async fn issue_session_after_cutoff<W, I, E>(
    unit_of_work: &W,
    token_issuer: &I,
    user_id: UserId,
    expected_updated_at: DateTime<Utc>,
    cutoff: DateTime<Utc>,
    client: SessionClient,
) -> Result<IssuedTokens, E>
where
    W: IdentityUnitOfWork,
    I: TokenIssuer,
    E: From<RepositoryError> + From<TokenIssuerError> + From<SessionRotationError>,
{
    wait_past_cutoff(cutoff).await;
    let mut tx = unit_of_work.begin().await?;
    let user = tx.load_user_for_update(&user_id).await?;
    let current = match user {
        Some(user)
            if !user.is_disabled()
                && !user.is_deleted()
                && same_stored_instant(user.updated_at(), expected_updated_at) =>
        {
            user
        }
        _ => return Err(SessionRotationError::Superseded.into()),
    };
    let tokens = token_issuer.issue(&user_id, current.is_verified()).await?;
    let refresh_token = RefreshToken::issue(
        tokens.refresh_token_id,
        user_id,
        tokens.refresh_token_expires_at,
        client.ip,
        client.user_agent,
    );
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

#[cfg(test)]
mod tests {
    use super::same_stored_instant;
    use chrono::{DateTime, Utc};

    #[test]
    fn a_stored_timestamp_matches_its_nanosecond_original() {
        let original = DateTime::<Utc>::from_timestamp(1_700_000_000, 123_456_789).unwrap();
        let stored = DateTime::<Utc>::from_timestamp_micros(original.timestamp_micros()).unwrap();
        assert_ne!(original, stored);
        assert!(same_stored_instant(original, stored));
        assert!(!same_stored_instant(original, stored + chrono::Duration::microseconds(1)));
    }
}
