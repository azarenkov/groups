use domain::user::role::UserRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenClaims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub token_type: TokenType,
    pub verified: bool,
    #[serde(default)]
    pub role: UserRole,
}

impl AccessTokenClaims {
    pub fn new(sub: Uuid, exp: i64, iat: i64, verified: bool, role: UserRole) -> Self {
        Self {
            sub,
            exp,
            iat,
            token_type: TokenType::Access,
            verified,
            role,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub jti: Uuid,
    pub token_type: TokenType,
}

impl RefreshTokenClaims {
    pub fn new(sub: Uuid, exp: i64, iat: i64, jti: Uuid) -> Self {
        Self {
            sub,
            exp,
            iat,
            jti,
            token_type: TokenType::Refresh,
        }
    }
}
