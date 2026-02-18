use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claims {
    pub id: Uuid,
    pub exp: usize,
    pub token_type: TokenType,
}

impl Claims {
    pub fn new(id: Uuid, exp: usize, token_type: TokenType) -> Self {
        Self {
            id,
            exp,
            token_type,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}
