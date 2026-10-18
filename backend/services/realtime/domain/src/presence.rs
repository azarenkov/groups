use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::envelope::Envelope;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PresenceUpdate {
    pub user_id: Uuid,
    pub online: bool,
}

impl PresenceUpdate {
    pub fn online(user_id: Uuid) -> Self {
        Self { user_id, online: true }
    }

    pub fn offline(user_id: Uuid) -> Self {
        Self { user_id, online: false }
    }

    pub fn envelope(&self) -> Envelope<'static> {
        Envelope::new(
            if self.online { "user.online" } else { "user.offline" },
            json!({ "userId": self.user_id }),
        )
    }

    pub fn is_stale_offline(&self, has_local_connections: bool, appear_offline: bool) -> bool {
        !self.online && has_local_connections && !appear_offline
    }
}

pub fn visible_presence(has_connections: bool, appear_offline: bool) -> bool {
    has_connections && !appear_offline
}

pub fn presence_after_preference(previously_hidden: Option<bool>, hidden: bool) -> Option<bool> {
    match previously_hidden {
        None => (!hidden).then_some(true),
        Some(previous) => (previous != hidden).then_some(!hidden),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newly_known_preference_only_announces_visible_users() {
        assert_eq!(presence_after_preference(None, false), Some(true));
        assert_eq!(presence_after_preference(None, true), None);
    }

    #[test]
    fn a_changed_preference_announces_the_new_visibility() {
        assert_eq!(presence_after_preference(Some(true), false), Some(true));
        assert_eq!(presence_after_preference(Some(false), true), Some(false));
        assert_eq!(presence_after_preference(Some(false), false), None);
        assert_eq!(presence_after_preference(Some(true), true), None);
    }
}
