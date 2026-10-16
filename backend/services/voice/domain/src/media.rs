pub mod repository;

use chrono::{DateTime, Utc};

pub fn issued_at(claimed: Option<DateTime<Utc>>, now: DateTime<Utc>) -> DateTime<Utc> {
    claimed.map_or(now, |at| at.min(now))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MediaState {
    mic_muted: bool,
    deafened: bool,
}

impl MediaState {
    pub fn new(mic_muted: bool, deafened: bool) -> Self {
        Self { mic_muted, deafened }
    }

    pub fn mic_muted(&self) -> bool {
        self.mic_muted
    }

    pub fn deafened(&self) -> bool {
        self.deafened
    }

    pub fn mic_live(&self) -> bool {
        !self.mic_muted && !self.deafened
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deafening_silences_the_microphone_but_keeps_its_own_toggle() {
        let state = MediaState::new(false, true);
        assert!(!state.mic_live());
        assert!(!state.mic_muted());
        assert!(state.deafened());
    }

    #[test]
    fn a_change_is_ordered_by_when_the_client_made_it_but_never_from_the_future() {
        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(2);
        assert_eq!(issued_at(Some(earlier), now), earlier);
        assert_eq!(issued_at(Some(now + chrono::Duration::hours(1)), now), now);
        assert_eq!(issued_at(None, now), now);
    }

    #[test]
    fn the_microphone_can_be_muted_without_deafening() {
        let state = MediaState::new(true, false);
        assert!(state.mic_muted());
        assert!(!state.deafened());
        assert_eq!(MediaState::default(), MediaState::new(false, false));
    }
}
