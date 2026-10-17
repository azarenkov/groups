use chrono::{DateTime, Utc};
use domain::user::id::UserId;

use crate::user::events::{UserEvent, lifecycle::UserPresencePreferenceChanged};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresencePreference {
    user_id: UserId,
    appear_offline: bool,
    updated_at: DateTime<Utc>,
}

impl PresencePreference {
    pub fn visible(user_id: UserId, now: DateTime<Utc>) -> Self {
        Self { user_id, appear_offline: false, updated_at: now }
    }

    pub fn restore(user_id: UserId, appear_offline: bool, updated_at: DateTime<Utc>) -> Self {
        Self { user_id, appear_offline, updated_at }
    }

    pub fn set_appear_offline(&mut self, appear_offline: bool, now: DateTime<Utc>) -> Option<UserEvent> {
        if self.appear_offline == appear_offline {
            return None;
        }
        self.appear_offline = appear_offline;
        self.updated_at = now;
        Some(UserEvent::PresencePreferenceChanged(UserPresencePreferenceChanged::record(self.user_id, appear_offline, now)))
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn appear_offline(&self) -> bool {
        self.appear_offline
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_real_change_is_recorded() {
        let now = Utc::now();
        let mut preference = PresencePreference::visible(UserId::generate(), now);
        assert!(preference.set_appear_offline(false, now).is_none());
        let event = preference.set_appear_offline(true, now).expect("hidden");
        assert!(matches!(event, UserEvent::PresencePreferenceChanged(e) if e.appear_offline && e.occurred_at == now));
        assert!(preference.appear_offline());
        assert!(preference.set_appear_offline(true, now).is_none());
    }
}
