use std::sync::Arc;

use chrono::Utc;

use crate::ports::captcha::{CaptchaChallenge, CaptchaCheckError, CaptchaStore, CaptchaVerifier};

#[derive(Debug, thiserror::Error)]
pub enum CaptchaError {
    #[error("Captcha required")]
    Required,
    #[error("Captcha invalid")]
    Invalid,
    #[error("Captcha expired")]
    Expired,
    #[error("Captcha store unavailable: {0}")]
    Unavailable(String),
}

impl From<CaptchaCheckError> for CaptchaError {
    fn from(error: CaptchaCheckError) -> Self {
        match error {
            CaptchaCheckError::Invalid => CaptchaError::Invalid,
            CaptchaCheckError::Expired => CaptchaError::Expired,
        }
    }
}

pub struct CaptchaGate<V, S>
where
    V: CaptchaVerifier,
    S: CaptchaStore,
{
    verifier: Arc<V>,
    store: Arc<S>,
    login_failure_threshold: u32,
}

impl<V, S> CaptchaGate<V, S>
where
    V: CaptchaVerifier,
    S: CaptchaStore,
{
    pub fn new(verifier: Arc<V>, store: Arc<S>, login_failure_threshold: u32) -> Self {
        Self {
            verifier,
            store,
            login_failure_threshold,
        }
    }

    pub fn challenge(&self) -> Option<CaptchaChallenge> {
        self.verifier
            .enabled()
            .then(|| self.verifier.issue(Utc::now()))
    }

    pub async fn require(&self, payload: Option<&str>) -> Result<(), CaptchaError> {
        if !self.verifier.enabled() {
            return Ok(());
        }
        let payload = payload
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .ok_or(CaptchaError::Required)?;
        let verified = self.verifier.verify(payload, Utc::now())?;
        let fresh = self
            .store
            .claim_solution(&verified.signature, verified.expires_at)
            .await
            .map_err(|e| CaptchaError::Unavailable(e.to_string()))?;
        if !fresh {
            return Err(CaptchaError::Invalid);
        }
        Ok(())
    }

    pub async fn guard_login(&self, keys: &[String], payload: Option<&str>) -> Result<(), CaptchaError> {
        if !self.verifier.enabled() {
            return Ok(());
        }
        let failures = match self.store.login_failures(keys).await {
            Ok(failures) => failures,
            Err(e) => {
                tracing::warn!(error = %e, "Login failure counter unavailable");
                0
            }
        };
        if failures < self.login_failure_threshold {
            return Ok(());
        }
        self.require(payload).await
    }

    pub async fn login_failed(&self, keys: &[String]) {
        if !self.verifier.enabled() {
            return;
        }
        if let Err(e) = self.store.record_login_failure(keys).await {
            tracing::warn!(error = %e, "Failed to record a login failure");
        }
    }

    pub async fn login_succeeded(&self, keys: &[String]) {
        if !self.verifier.enabled() {
            return;
        }
        if let Err(e) = self.store.clear_login_failures(keys).await {
            tracing::warn!(error = %e, "Failed to reset login failures");
        }
    }
}

pub fn login_keys(ip: Option<&str>, email: &str) -> Vec<String> {
    let mut keys = vec![format!("email:{}", email.trim().to_lowercase())];
    if let Some(ip) = ip.map(str::trim).filter(|ip| !ip.is_empty()) {
        keys.push(format!("ip:{ip}"));
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::captcha::{CaptchaStoreError, VerifiedCaptcha};
    use async_trait::async_trait;
    use chrono::{DateTime, Duration};
    use std::collections::{HashMap, HashSet};
    use tokio::sync::Mutex;

    struct FakeVerifier {
        enabled: bool,
    }

    impl CaptchaVerifier for FakeVerifier {
        fn enabled(&self) -> bool {
            self.enabled
        }

        fn issue(&self, _now: DateTime<Utc>) -> CaptchaChallenge {
            CaptchaChallenge {
                algorithm: "SHA-256".into(),
                challenge: "c".into(),
                max_number: 10,
                salt: "s".into(),
                signature: "sig".into(),
            }
        }

        fn verify(&self, payload: &str, now: DateTime<Utc>) -> Result<VerifiedCaptcha, CaptchaCheckError> {
            match payload {
                "expired" => Err(CaptchaCheckError::Expired),
                "bad" => Err(CaptchaCheckError::Invalid),
                other => Ok(VerifiedCaptcha {
                    signature: other.to_string(),
                    expires_at: now + Duration::minutes(5),
                }),
            }
        }
    }

    #[derive(Default)]
    struct FakeStore {
        used: Mutex<HashSet<String>>,
        failures: Mutex<HashMap<String, u32>>,
    }

    #[async_trait]
    impl CaptchaStore for FakeStore {
        async fn claim_solution(&self, signature: &str, _expires_at: DateTime<Utc>) -> Result<bool, CaptchaStoreError> {
            Ok(self.used.lock().await.insert(signature.to_string()))
        }

        async fn login_failures(&self, keys: &[String]) -> Result<u32, CaptchaStoreError> {
            let failures = self.failures.lock().await;
            Ok(keys.iter().filter_map(|k| failures.get(k)).copied().max().unwrap_or(0))
        }

        async fn record_login_failure(&self, keys: &[String]) -> Result<(), CaptchaStoreError> {
            let mut failures = self.failures.lock().await;
            for key in keys {
                *failures.entry(key.clone()).or_default() += 1;
            }
            Ok(())
        }

        async fn clear_login_failures(&self, keys: &[String]) -> Result<(), CaptchaStoreError> {
            let mut failures = self.failures.lock().await;
            for key in keys {
                failures.remove(key);
            }
            Ok(())
        }
    }

    fn gate(enabled: bool) -> CaptchaGate<FakeVerifier, FakeStore> {
        CaptchaGate::new(Arc::new(FakeVerifier { enabled }), Arc::new(FakeStore::default()), 3)
    }

    #[tokio::test]
    async fn require_rejects_missing_forged_and_expired_solutions() {
        let gate = gate(true);
        assert!(matches!(gate.require(None).await, Err(CaptchaError::Required)));
        assert!(matches!(gate.require(Some("  ")).await, Err(CaptchaError::Required)));
        assert!(matches!(gate.require(Some("bad")).await, Err(CaptchaError::Invalid)));
        assert!(matches!(gate.require(Some("expired")).await, Err(CaptchaError::Expired)));
    }

    #[tokio::test]
    async fn a_solution_is_accepted_only_once() {
        let gate = gate(true);
        assert!(gate.require(Some("one")).await.is_ok());
        assert!(matches!(gate.require(Some("one")).await, Err(CaptchaError::Invalid)));
        assert!(gate.require(Some("two")).await.is_ok());
    }

    #[tokio::test]
    async fn disabled_gate_lets_everything_through() {
        let gate = gate(false);
        assert!(gate.challenge().is_none());
        assert!(gate.require(None).await.is_ok());
    }

    #[tokio::test]
    async fn login_needs_captcha_after_repeated_failures_until_success() {
        let gate = gate(true);
        let keys = login_keys(Some("10.0.0.1"), "User@Example.com");
        for _ in 0..3 {
            assert!(gate.guard_login(&keys, None).await.is_ok());
            gate.login_failed(&keys).await;
        }
        assert!(matches!(gate.guard_login(&keys, None).await, Err(CaptchaError::Required)));
        assert!(gate.guard_login(&keys, Some("solved")).await.is_ok());
        gate.login_succeeded(&keys).await;
        assert!(gate.guard_login(&keys, None).await.is_ok());
    }

    #[test]
    fn login_keys_normalize_email_and_skip_missing_ip() {
        assert_eq!(login_keys(None, " A@B.c "), vec!["email:a@b.c".to_string()]);
        assert_eq!(
            login_keys(Some("1.2.3.4"), "a@b.c"),
            vec!["email:a@b.c".to_string(), "ip:1.2.3.4".to_string()]
        );
    }
}
