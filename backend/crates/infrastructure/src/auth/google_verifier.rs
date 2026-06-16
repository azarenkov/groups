use std::time::{Duration, Instant};

use application::ports::oauth_verifier::{
    OAuthIdentity, OAuthIdentityVerifier, OAuthProvider, OAuthVerifyError,
};
use async_trait::async_trait;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use tokio::sync::{Mutex, RwLock};

const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const GOOGLE_ISSUERS: [&str; 2] = ["accounts.google.com", "https://accounts.google.com"];
const JWKS_TTL: Duration = Duration::from_secs(60 * 60);
const JWKS_REFETCH_COOLDOWN: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct GoogleClaims {
    sub: String,
    email: String,
    #[serde(default, deserialize_with = "de_bool_flexible")]
    email_verified: bool,
    #[serde(default)]
    name: Option<String>,
}

fn de_bool_flexible<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::String(s) => Ok(s == "true"),
        _ => Err(D::Error::custom("invalid email_verified value")),
    }
}

struct CachedJwks {
    keys: Vec<Jwk>,
    fetched_at: Instant,
}

pub struct GoogleIdTokenVerifier {
    client_id: String,
    http: reqwest::Client,
    cache: RwLock<Option<CachedJwks>>,
    last_unknown_refetch: Mutex<Option<Instant>>,
}

impl GoogleIdTokenVerifier {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            http: reqwest::Client::new(),
            cache: RwLock::new(None),
            last_unknown_refetch: Mutex::new(None),
        }
    }

    async fn fetch_jwks(&self) -> Result<Vec<Jwk>, OAuthVerifyError> {
        let jwks: Jwks = self
            .http
            .get(GOOGLE_JWKS_URL)
            .send()
            .await
            .map_err(|e| OAuthVerifyError::Transport(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthVerifyError::Transport(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthVerifyError::Transport(e.to_string()))?;
        Ok(jwks.keys)
    }

    async fn key_for(&self, kid: &str) -> Result<Jwk, OAuthVerifyError> {
        let mut cache_is_fresh = false;
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.as_ref() {
                let fresh = cached.fetched_at.elapsed() < JWKS_TTL;
                if fresh && let Some(found) = cached.keys.iter().find(|k| k.kid == kid) {
                    return Ok(found.clone());
                }
                cache_is_fresh = fresh;
            }
        }

        if cache_is_fresh {
            let last = self.last_unknown_refetch.lock().await;
            if let Some(at) = *last
                && at.elapsed() < JWKS_REFETCH_COOLDOWN
            {
                return Err(OAuthVerifyError::InvalidToken("unknown key id".to_string()));
            }
        }

        let keys = self.fetch_jwks().await?;

        if cache_is_fresh {
            *self.last_unknown_refetch.lock().await = Some(Instant::now());
        }

        let found = keys.iter().find(|k| k.kid == kid).cloned();
        {
            let mut cache = self.cache.write().await;
            *cache = Some(CachedJwks {
                keys,
                fetched_at: Instant::now(),
            });
        }
        found.ok_or_else(|| OAuthVerifyError::InvalidToken("unknown key id".to_string()))
    }
}

#[async_trait]
impl OAuthIdentityVerifier for GoogleIdTokenVerifier {
    async fn verify(
        &self,
        provider: OAuthProvider,
        token: &str,
    ) -> Result<OAuthIdentity, OAuthVerifyError> {
        if provider != OAuthProvider::Google {
            return Err(OAuthVerifyError::UnsupportedProvider(provider.as_str()));
        }
        if self.client_id.is_empty() {
            return Err(OAuthVerifyError::NotConfigured(OAuthProvider::Google.as_str()));
        }

        let header = decode_header(token)
            .map_err(|e| OAuthVerifyError::InvalidToken(e.to_string()))?;
        let kid = header
            .kid
            .ok_or_else(|| OAuthVerifyError::InvalidToken("missing key id".to_string()))?;

        let jwk = self.key_for(&kid).await?;
        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
            .map_err(|e| OAuthVerifyError::InvalidToken(e.to_string()))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[self.client_id.as_str()]);
        validation.set_issuer(&GOOGLE_ISSUERS);

        let data = decode::<GoogleClaims>(token, &decoding_key, &validation)
            .map_err(|e| OAuthVerifyError::InvalidToken(e.to_string()))?;

        Ok(OAuthIdentity {
            provider: OAuthProvider::Google,
            subject: data.claims.sub,
            email: data.claims.email,
            email_verified: data.claims.email_verified,
            name: data.claims.name,
        })
    }
}
