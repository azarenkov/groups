use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, TimeZone, Utc};
use hmac::{Hmac, Mac};
use identity::ports::captcha::{CaptchaChallenge, CaptchaCheckError, CaptchaVerifier, VerifiedCaptcha};
use rand::RngExt;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const ALGORITHM: &str = "SHA-256";
const MIN_MAX_NUMBER: u64 = 1_000;
const MAX_MAX_NUMBER: u64 = 10_000_000;
const MIN_EXPIRES_SECS: i64 = 30;
const MAX_EXPIRES_SECS: i64 = 60 * 60;
const MAX_PAYLOAD_LEN: usize = 2048;
const MIN_KEY_LEN: usize = 32;

#[derive(Debug, Clone, Deserialize)]
pub struct AltchaConfig {
    #[serde(default)]
    pub altcha_hmac_key: Option<String>,
    #[serde(default = "default_max_number")]
    pub altcha_max_number: u64,
    #[serde(default = "default_expires_secs")]
    pub altcha_expires_secs: i64,
    #[serde(default = "default_login_failures")]
    pub altcha_login_failures: u32,
    #[serde(default = "default_login_window_secs")]
    pub altcha_login_window_secs: u64,
}

fn default_max_number() -> u64 {
    50_000
}

fn default_expires_secs() -> i64 {
    300
}

fn default_login_failures() -> u32 {
    3
}

fn default_login_window_secs() -> u64 {
    15 * 60
}

impl AltchaConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}

#[derive(Deserialize)]
struct Solution {
    algorithm: String,
    challenge: String,
    number: u64,
    salt: String,
    signature: String,
}

pub struct AltchaVerifier {
    key: Option<Vec<u8>>,
    max_number: u64,
    expires: Duration,
}

impl AltchaVerifier {
    pub fn new(key: Option<&str>, max_number: u64, expires_secs: i64) -> Self {
        let key = key.map(str::trim).filter(|k| !k.is_empty()).and_then(|k| {
            if k.len() < MIN_KEY_LEN {
                tracing::error!("ALTCHA_HMAC_KEY is shorter than {MIN_KEY_LEN} characters; captcha is disabled");
                return None;
            }
            Some(k.as_bytes().to_vec())
        });
        if key.is_none() {
            tracing::warn!("ALTCHA_HMAC_KEY is not set; captcha is disabled");
        }
        Self {
            key,
            max_number: max_number.clamp(MIN_MAX_NUMBER, MAX_MAX_NUMBER),
            expires: Duration::seconds(expires_secs.clamp(MIN_EXPIRES_SECS, MAX_EXPIRES_SECS)),
        }
    }

    pub fn from_config(config: &AltchaConfig) -> Self {
        Self::new(
            config.altcha_hmac_key.as_deref(),
            config.altcha_max_number,
            config.altcha_expires_secs,
        )
    }

    fn mac(key: &[u8]) -> Hmac<Sha256> {
        <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts keys of any length")
    }

    fn hash(salt: &str, number: u64) -> String {
        hex::encode(Sha256::digest(format!("{salt}{number}").as_bytes()))
    }

    fn sign(key: &[u8], challenge: &str) -> String {
        let mut mac = Self::mac(key);
        mac.update(challenge.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    fn expires_at(salt: &str) -> Option<DateTime<Utc>> {
        let (_, query) = salt.split_once('?')?;
        let value = query
            .split('&')
            .find_map(|pair| pair.strip_prefix("expires="))?;
        Utc.timestamp_opt(value.parse().ok()?, 0).single()
    }

    fn decode(payload: &str) -> Option<Solution> {
        if payload.len() > MAX_PAYLOAD_LEN {
            return None;
        }
        let bytes = STANDARD
            .decode(payload)
            .or_else(|_| URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')))
            .ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

impl CaptchaVerifier for AltchaVerifier {
    fn enabled(&self) -> bool {
        self.key.is_some()
    }

    fn issue(&self, now: DateTime<Utc>) -> CaptchaChallenge {
        let key = self.key.as_deref().unwrap_or_default();
        let mut rng = rand::rng();
        let nonce: [u8; 12] = rng.random();
        let salt = format!("{}?expires={}", hex::encode(nonce), (now + self.expires).timestamp());
        let number = rng.random_range(0..=self.max_number);
        let challenge = Self::hash(&salt, number);
        let signature = Self::sign(key, &challenge);
        CaptchaChallenge {
            algorithm: ALGORITHM.to_string(),
            challenge,
            max_number: self.max_number,
            salt,
            signature,
        }
    }

    fn verify(&self, payload: &str, now: DateTime<Utc>) -> Result<VerifiedCaptcha, CaptchaCheckError> {
        let key = self.key.as_deref().ok_or(CaptchaCheckError::Invalid)?;
        let solution = Self::decode(payload).ok_or(CaptchaCheckError::Invalid)?;
        if solution.algorithm != ALGORITHM {
            return Err(CaptchaCheckError::Invalid);
        }
        let signature = hex::decode(&solution.signature).map_err(|_| CaptchaCheckError::Invalid)?;
        let mut mac = Self::mac(key);
        mac.update(solution.challenge.as_bytes());
        mac.verify_slice(&signature)
            .map_err(|_| CaptchaCheckError::Invalid)?;
        let expires_at = Self::expires_at(&solution.salt).ok_or(CaptchaCheckError::Invalid)?;
        if expires_at <= now {
            return Err(CaptchaCheckError::Expired);
        }
        if Self::hash(&solution.salt, solution.number) != solution.challenge {
            return Err(CaptchaCheckError::Invalid);
        }
        Ok(VerifiedCaptcha {
            signature: solution.signature.to_lowercase(),
            expires_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef";

    fn verifier() -> AltchaVerifier {
        AltchaVerifier::new(Some(KEY), 1_000, 300)
    }

    fn solve(challenge: &CaptchaChallenge) -> u64 {
        (0..=challenge.max_number)
            .find(|n| AltchaVerifier::hash(&challenge.salt, *n) == challenge.challenge)
            .expect("challenge is solvable")
    }

    fn payload(challenge: &CaptchaChallenge, number: u64) -> String {
        STANDARD.encode(
            serde_json::json!({
                "algorithm": challenge.algorithm,
                "challenge": challenge.challenge,
                "number": number,
                "salt": challenge.salt,
                "signature": challenge.signature,
            })
            .to_string(),
        )
    }

    #[test]
    fn accepts_a_correct_solution() {
        let now = Utc::now();
        let verifier = verifier();
        let challenge = verifier.issue(now);
        let verified = verifier.verify(&payload(&challenge, solve(&challenge)), now).unwrap();
        assert_eq!(verified.signature, challenge.signature);
        assert_eq!(verified.expires_at.timestamp(), (now + Duration::seconds(300)).timestamp());
    }

    #[test]
    fn rejects_an_expired_solution() {
        let now = Utc::now();
        let verifier = verifier();
        let challenge = verifier.issue(now);
        let later = now + Duration::seconds(301);
        assert_eq!(
            verifier.verify(&payload(&challenge, solve(&challenge)), later),
            Err(CaptchaCheckError::Expired)
        );
    }

    #[test]
    fn rejects_a_wrong_number() {
        let now = Utc::now();
        let verifier = verifier();
        let challenge = verifier.issue(now);
        let wrong = (solve(&challenge) + 1) % (challenge.max_number + 1);
        assert_eq!(
            verifier.verify(&payload(&challenge, wrong), now),
            Err(CaptchaCheckError::Invalid)
        );
    }

    #[test]
    fn rejects_a_forged_challenge() {
        let now = Utc::now();
        let verifier = verifier();
        let mut forged = verifier.issue(now);
        forged.salt = format!("abc?expires={}", (now + Duration::hours(1)).timestamp());
        forged.challenge = AltchaVerifier::hash(&forged.salt, 0);
        assert_eq!(verifier.verify(&payload(&forged, 0), now), Err(CaptchaCheckError::Invalid));
    }

    #[test]
    fn rejects_a_challenge_signed_with_another_key() {
        let now = Utc::now();
        let other = AltchaVerifier::new(Some("fedcba9876543210fedcba9876543210"), 1_000, 300);
        let challenge = other.issue(now);
        assert_eq!(
            verifier().verify(&payload(&challenge, solve(&challenge)), now),
            Err(CaptchaCheckError::Invalid)
        );
    }

    #[test]
    fn rejects_an_extended_expiry() {
        let now = Utc::now();
        let verifier = verifier();
        let challenge = verifier.issue(now);
        let number = solve(&challenge);
        let mut tampered = challenge.clone();
        tampered.salt = tampered.salt.replace("?expires=", "?expires=9");
        assert_eq!(
            verifier.verify(&payload(&tampered, number), now),
            Err(CaptchaCheckError::Invalid)
        );
    }

    #[test]
    fn rejects_garbage() {
        let verifier = verifier();
        let now = Utc::now();
        assert_eq!(verifier.verify("not base64!", now), Err(CaptchaCheckError::Invalid));
        assert_eq!(verifier.verify(&STANDARD.encode("{}"), now), Err(CaptchaCheckError::Invalid));
        assert_eq!(verifier.verify(&"A".repeat(4096), now), Err(CaptchaCheckError::Invalid));
    }

    #[test]
    fn short_or_missing_key_disables_the_captcha() {
        assert!(!AltchaVerifier::new(None, 1_000, 300).enabled());
        assert!(!AltchaVerifier::new(Some("short"), 1_000, 300).enabled());
        assert!(verifier().enabled());
    }
}
