use std::net::{IpAddr, SocketAddr};
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

    pub async fn guard_login(&self, keys: &LoginKeys, payload: Option<&str>) -> Result<(), CaptchaError> {
        if !self.verifier.enabled() {
            return Ok(());
        }
        let attempts = match self.store.register_login_attempt(&keys.all()).await {
            Ok(attempts) => attempts,
            Err(e) => {
                tracing::warn!(error = %e, "Login attempt counter unavailable");
                0
            }
        };
        if attempts <= self.login_failure_threshold {
            return Ok(());
        }
        self.require(payload).await
    }

    pub async fn login_succeeded(&self, keys: &LoginKeys) {
        if !self.verifier.enabled() {
            return;
        }
        if let Err(e) = self
            .store
            .clear_login_attempts(std::slice::from_ref(&keys.account))
            .await
        {
            tracing::warn!(error = %e, "Failed to reset account login attempts");
        }
        if let Some(ip) = &keys.ip
            && let Err(e) = self
                .store
                .release_login_attempt(std::slice::from_ref(ip))
                .await
        {
            tracing::warn!(error = %e, "Failed to release an IP login attempt");
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginKeys {
    pub account: String,
    pub ip: Option<String>,
}

impl LoginKeys {
    pub fn new(ip: Option<&str>, email: &str) -> Self {
        Self {
            account: format!("email:{}", email.trim().to_lowercase()),
            ip: ip.and_then(normalize_ip).map(|ip| format!("ip:{ip}")),
        }
    }

    fn all(&self) -> Vec<String> {
        std::iter::once(self.account.clone())
            .chain(self.ip.clone())
            .collect()
    }
}

fn normalize_ip(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let ip = raw
        .parse::<SocketAddr>()
        .map(|addr| addr.ip())
        .or_else(|_| {
            raw.trim_start_matches('[')
                .trim_end_matches(']')
                .parse::<IpAddr>()
        })
        .ok()?;
    Some(match ip.to_canonical() {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
        }
    })
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
        attempts: Mutex<HashMap<String, u32>>,
    }

    #[async_trait]
    impl CaptchaStore for FakeStore {
        async fn claim_solution(&self, signature: &str, _expires_at: DateTime<Utc>) -> Result<bool, CaptchaStoreError> {
            Ok(self.used.lock().await.insert(signature.to_string()))
        }

        async fn register_login_attempt(&self, keys: &[String]) -> Result<u32, CaptchaStoreError> {
            let mut attempts = self.attempts.lock().await;
            let mut max = 0;
            for key in keys {
                let count = attempts.entry(key.clone()).or_default();
                *count += 1;
                max = max.max(*count);
            }
            Ok(max)
        }

        async fn release_login_attempt(&self, keys: &[String]) -> Result<(), CaptchaStoreError> {
            let mut attempts = self.attempts.lock().await;
            for key in keys {
                if let Some(count) = attempts.get_mut(key) {
                    *count = count.saturating_sub(1);
                }
            }
            Ok(())
        }

        async fn clear_login_attempts(&self, keys: &[String]) -> Result<(), CaptchaStoreError> {
            let mut attempts = self.attempts.lock().await;
            for key in keys {
                attempts.remove(key);
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
        let keys = LoginKeys::new(None, "a@b.c");
        for _ in 0..10 {
            assert!(gate.guard_login(&keys, None).await.is_ok());
        }
    }

    #[tokio::test]
    async fn attempts_are_counted_before_the_password_is_checked() {
        let gate = gate(true);
        let keys = LoginKeys::new(Some("10.0.0.1:5000"), "User@Example.com");
        for _ in 0..3 {
            assert!(gate.guard_login(&keys, None).await.is_ok());
        }
        assert!(matches!(gate.guard_login(&keys, None).await, Err(CaptchaError::Required)));
        assert!(gate.guard_login(&keys, Some("solved")).await.is_ok());
    }

    #[tokio::test]
    async fn success_resets_the_account_but_keeps_other_ip_failures() {
        let gate = gate(true);
        for target in ["a@x.io", "b@x.io", "c@x.io"] {
            let keys = LoginKeys::new(Some("10.0.0.1"), target);
            assert!(gate.guard_login(&keys, None).await.is_ok());
        }
        let own = LoginKeys::new(Some("10.0.0.1"), "own@example.com");
        assert!(matches!(gate.guard_login(&own, None).await, Err(CaptchaError::Required)));
        assert!(gate.guard_login(&own, Some("solved")).await.is_ok());
        gate.login_succeeded(&own).await;
        let next = LoginKeys::new(Some("10.0.0.1"), "d@x.io");
        assert!(matches!(gate.guard_login(&next, None).await, Err(CaptchaError::Required)));
    }

    #[tokio::test]
    async fn successful_logins_do_not_pile_up_on_a_shared_ip() {
        let gate = gate(true);
        for i in 0..10 {
            let keys = LoginKeys::new(Some("10.0.0.1"), &format!("user{i}@x.io"));
            assert!(gate.guard_login(&keys, None).await.is_ok());
            gate.login_succeeded(&keys).await;
        }
    }

    #[test]
    fn login_keys_normalize_email_and_ip() {
        let keys = LoginKeys::new(None, " A@B.c ");
        assert_eq!(keys.account, "email:a@b.c");
        assert_eq!(keys.ip, None);
        assert_eq!(LoginKeys::new(Some("1.2.3.4:5555"), "a@b.c").ip.as_deref(), Some("ip:1.2.3.4"));
        assert_eq!(LoginKeys::new(Some("1.2.3.4"), "a@b.c").ip.as_deref(), Some("ip:1.2.3.4"));
        assert_eq!(
            LoginKeys::new(Some("[2001:db8:1:2:aa::1]:443"), "a@b.c").ip.as_deref(),
            Some("ip:2001:db8:1:2::/64")
        );
        assert_eq!(
            LoginKeys::new(Some("::ffff:10.1.2.3"), "a@b.c").ip.as_deref(),
            Some("ip:10.1.2.3")
        );
        assert_eq!(LoginKeys::new(Some("garbage"), "a@b.c").ip, None);
    }
}
