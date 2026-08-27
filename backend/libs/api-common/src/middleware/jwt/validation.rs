use jsonwebtoken::{DecodingKey, Validation, decode};
use uuid::Uuid;

use crate::middleware::jwt::claims::{AccessTokenClaims, TokenType};

pub fn validate_access_token(token: &str, secret: &str) -> anyhow::Result<(Uuid, bool)> {
    let decoded = decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )?;

    if decoded.claims.token_type != TokenType::Access {
        return Err(anyhow::anyhow!("Invalid token type"));
    }

    Ok((decoded.claims.sub, decoded.claims.verified))
}
