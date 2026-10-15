use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallStatus {
    Completed { duration_seconds: i32 },
    Declined,
    Missed,
}

impl CallStatus {
    pub const UNREAD: &'static [&'static str] = &[Self::Missed.as_str()];

    pub fn resolve(answered_at: Option<DateTime<Utc>>, ended_at: DateTime<Utc>, declined: bool) -> Self {
        match answered_at {
            Some(answered_at) => Self::Completed {
                duration_seconds: (ended_at - answered_at).num_seconds().clamp(0, i64::from(i32::MAX)) as i32,
            },
            None if declined => Self::Declined,
            None => Self::Missed,
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Completed { .. } => "completed",
            Self::Declined => "declined",
            Self::Missed => "missed",
        }
    }

    pub fn duration_seconds(&self) -> Option<i32> {
        match self {
            Self::Completed { duration_seconds } => Some(*duration_seconds),
            Self::Declined | Self::Missed => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn an_answered_call_is_completed_with_its_duration() {
        let answered = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        assert_eq!(
            CallStatus::resolve(Some(answered), answered + Duration::seconds(95), false),
            CallStatus::Completed { duration_seconds: 95 }
        );
        assert_eq!(
            CallStatus::resolve(Some(answered), answered - Duration::seconds(5), false),
            CallStatus::Completed { duration_seconds: 0 }
        );
    }

    #[test]
    fn an_unanswered_call_is_declined_or_missed() {
        let now = DateTime::from_timestamp(1_800_000_000, 0).unwrap();
        assert_eq!(CallStatus::resolve(None, now, true), CallStatus::Declined);
        assert_eq!(CallStatus::resolve(None, now, false), CallStatus::Missed);
    }

    #[test]
    fn only_missed_calls_count_as_unread() {
        assert_eq!(CallStatus::UNREAD, &["missed"]);
    }
}
