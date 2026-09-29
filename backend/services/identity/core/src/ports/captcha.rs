use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptchaChallenge {
    pub algorithm: String,
    pub challenge: String,
    pub max_number: u64,
    pub salt: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCaptcha {
    pub signature: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaptchaCheckError {
    #[error("captcha solution is invalid")]
    Invalid,
    #[error("captcha challenge has expired")]
    Expired,
}

pub trait CaptchaVerifier: Send + Sync {
    fn enabled(&self) -> bool;
    fn issue(&self, now: DateTime<Utc>) -> CaptchaChallenge;
    fn verify(&self, payload: &str, now: DateTime<Utc>) -> Result<VerifiedCaptcha, CaptchaCheckError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CaptchaStoreError {
    #[error("captcha store error: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait CaptchaStore: Send + Sync {
    async fn claim_solution(&self, signature: &str, expires_at: DateTime<Utc>) -> Result<bool, CaptchaStoreError>;
    async fn register_login_attempt(&self, keys: &[String]) -> Result<u32, CaptchaStoreError>;
    async fn release_login_attempt(&self, keys: &[String]) -> Result<(), CaptchaStoreError>;
    async fn clear_login_attempts(&self, keys: &[String]) -> Result<(), CaptchaStoreError>;
}
