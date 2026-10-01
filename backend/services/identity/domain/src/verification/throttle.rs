use chrono::{DateTime, Duration, Utc};

pub const RESEND_COOLDOWN_SECS: i64 = 60;
pub const HOURLY_CODE_LIMIT: i64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IssuedCodes {
    pub last_issued_at: Option<DateTime<Utc>>,
    pub oldest_in_window: Option<DateTime<Utc>>,
    pub issued_last_hour: i64,
}

impl IssuedCodes {
    pub fn window_start(now: DateTime<Utc>) -> DateTime<Utc> {
        now - Self::window()
    }

    fn window() -> Duration {
        Duration::hours(1)
    }

    pub fn retry_after(&self, now: DateTime<Utc>) -> Option<Duration> {
        if let Some(last) = self.last_issued_at {
            let ready = last + Duration::seconds(RESEND_COOLDOWN_SECS);
            if ready > now {
                return Some(ready - now);
            }
        }
        if self.issued_last_hour >= HOURLY_CODE_LIMIT {
            let frees_at = self
                .oldest_in_window
                .map(|oldest| oldest + Self::window())
                .unwrap_or(now + Self::window());
            return Some((frees_at - now).max(Duration::seconds(1)));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_user_can_get_a_code() {
        assert_eq!(IssuedCodes::default().retry_after(Utc::now()), None);
    }

    #[test]
    fn the_next_code_waits_for_the_cooldown() {
        let now = Utc::now();
        let codes = IssuedCodes {
            last_issued_at: Some(now - Duration::seconds(20)),
            oldest_in_window: Some(now - Duration::seconds(20)),
            issued_last_hour: 1,
        };
        assert_eq!(codes.retry_after(now), Some(Duration::seconds(40)));
        assert_eq!(codes.retry_after(now + Duration::seconds(40)), None);
    }

    #[test]
    fn the_hourly_limit_holds_even_after_the_cooldown() {
        let now = Utc::now();
        let codes = IssuedCodes {
            last_issued_at: Some(now - Duration::minutes(5)),
            oldest_in_window: Some(now - Duration::minutes(50)),
            issued_last_hour: HOURLY_CODE_LIMIT,
        };
        assert_eq!(codes.retry_after(now), Some(Duration::minutes(10)));
        let early = IssuedCodes {
            oldest_in_window: Some(now - Duration::minutes(5)),
            ..codes
        };
        assert_eq!(early.retry_after(now), Some(Duration::minutes(55)));
    }
}
