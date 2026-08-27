use identity::ports::token_issuer::{
    DecodedRefreshToken, IssuedTokens, TokenIssuer, TokenIssuerError,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use domain::{refresh_token::id::RefreshTokenId, user::id::UserId};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use shared::middleware::jwt::claims::{AccessTokenClaims, RefreshTokenClaims, TokenType};
use uuid::Uuid;

const ACCESS_TOKEN_EXPIRATION_MINUTES: i64 = 15;
const REFRESH_TOKEN_EXPIRATION_DAYS: i64 = 30;

pub struct JwtTokenIssuer {
    secret: String,
}

impl JwtTokenIssuer {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    fn encoding_key(&self) -> EncodingKey {
        EncodingKey::from_secret(self.secret.as_bytes())
    }

    fn decoding_key(&self) -> DecodingKey {
        DecodingKey::from_secret(self.secret.as_bytes())
    }

    fn encode_access(
        &self,
        user_id: Uuid,
        now: DateTime<Utc>,
        verified: bool,
    ) -> Result<String, TokenIssuerError> {
        let exp = (now + Duration::minutes(ACCESS_TOKEN_EXPIRATION_MINUTES)).timestamp();
        let claims = AccessTokenClaims::new(user_id, exp, now.timestamp(), verified);

        encode(&Header::default(), &claims, &self.encoding_key())
            .map_err(|e| TokenIssuerError::Encoding(e.to_string()))
    }

    fn encode_refresh(
        &self,
        user_id: Uuid,
        jti: Uuid,
        now: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<String, TokenIssuerError> {
        let claims =
            RefreshTokenClaims::new(user_id, expires_at.timestamp(), now.timestamp(), jti);

        encode(&Header::default(), &claims, &self.encoding_key())
            .map_err(|e| TokenIssuerError::Encoding(e.to_string()))
    }
}

#[async_trait]
impl TokenIssuer for JwtTokenIssuer {
    async fn issue(
        &self,
        user_id: &UserId,
        verified: bool,
    ) -> Result<IssuedTokens, TokenIssuerError> {
        let now = Utc::now();
        let refresh_expires_at = now + Duration::days(REFRESH_TOKEN_EXPIRATION_DAYS);
        let refresh_id = RefreshTokenId::generate();

        let access_token = self.encode_access(user_id.uuid(), now, verified)?;
        let refresh_token =
            self.encode_refresh(user_id.uuid(), refresh_id.uuid(), now, refresh_expires_at)?;

        Ok(IssuedTokens {
            access_token,
            refresh_token,
            refresh_token_id: refresh_id,
            refresh_token_expires_at: refresh_expires_at,
            access_token_expires_in: ACCESS_TOKEN_EXPIRATION_MINUTES * 60,
        })
    }

    fn decode_refresh(&self, token: &str) -> Result<DecodedRefreshToken, TokenIssuerError> {
        let decoded = decode::<RefreshTokenClaims>(
            token,
            &self.decoding_key(),
            &Validation::new(jsonwebtoken::Algorithm::HS256),
        )
        .map_err(|e| TokenIssuerError::Decoding(e.to_string()))?;

        if decoded.claims.token_type != TokenType::Refresh {
            return Err(TokenIssuerError::InvalidTokenType);
        }

        let expires_at = DateTime::<Utc>::from_timestamp(decoded.claims.exp, 0)
            .ok_or_else(|| TokenIssuerError::Decoding("invalid exp claim".to_string()))?;

        Ok(DecodedRefreshToken {
            user_id: UserId::new(decoded.claims.sub),
            refresh_token_id: RefreshTokenId::new(decoded.claims.jti),
            expires_at,
        })
    }
}
