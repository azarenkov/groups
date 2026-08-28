use jsonwebtoken::{DecodingKey, Validation, decode};
use uuid::Uuid;

use crate::middleware::jwt::claims::{AccessTokenClaims, TokenType};

pub struct ValidatedAccess {
    pub user_id: Uuid,
    pub verified: bool,
    pub issued_at: i64,
}

pub fn validate_access_token(token: &str, secret: &str) -> anyhow::Result<ValidatedAccess> {
    let decoded = decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )?;

    if decoded.claims.token_type != TokenType::Access {
        return Err(anyhow::anyhow!("Invalid token type"));
    }

    Ok(ValidatedAccess {
        user_id: decoded.claims.sub,
        verified: decoded.claims.verified,
        issued_at: decoded.claims.iat,
    })
}
